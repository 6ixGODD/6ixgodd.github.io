NumPy: ufunc 与向量化
=======================================================================

:date: 2026-09-30
:slug: numpy-ufunc-vectorization
:tags: numpy, zh
:lang: zh
:draft: false

前两篇分别讨论了数组的内存布局，以及广播如何为不同形状的输入建立索引对应关系。对于 ``a + b``，这已经说明每个结果需要读取哪些位置，但尚未涉及加法的执行过程。

本文以 ``np.add`` 为例，分析类型选择、步长迭代与内层循环，并通过实验区分数组级向量化、Python 函数包装和 SIMD。实验及源码仍固定到 NumPy 2.3.5；除专门的对照实验外，讨论对象均为普通数值 ``ndarray``，不涉及子类或第三方数组的重载行为。


1. 数组运算与 ufunc
-----------------------------------------------------------------------

沿用上一篇的二维数组与一维数组相加，类型改为 ``float64``，便于后面对照浮点循环：

.. code-block:: python

   import numpy as np

   a = np.arange(12, dtype=np.float64).reshape(3, 4)
   b = np.array([10, 20, 30, 40], dtype=np.float64)
   result = np.add(a, b)

   expected = np.empty_like(a)
   for i in range(a.shape[0]):
       for j in range(a.shape[1]):
           expected[i, j] = a[i, j] + b[j]

   assert np.array_equal(result, expected)

两种写法都计算十二次加法，算法的工作量没有从 ``O(N)`` 变成常数。区别在于循环由谁执行：后一种写法在 Python 中逐个索引、运算并赋值，前一种将整个数组交给 NumPy。[ufunc-basics]_

这里的 ``np.add`` 不是一个普通 Python 函数：

.. code-block:: python

   print(type(np.add))              # <class 'numpy.ufunc'>
   print(np.add.nin, np.add.nout)    # 2 1

``ufunc`` 是 universal function 的简称。对于 ``np.add`` 这类逐元素运算，它将固定数量的输入、输出与广播、类型转换、循环执行等机制组织在一起。``nin`` 和 ``nout`` 分别表示输入和输出操作数的数量，与数组中有多少个元素无关。[ufunc-reference]_

对本文的普通数组，``a + b`` 使用的也是加法 ufunc。写成 ``np.add(a, b)`` 主要是为了显式使用它的接口，而不是另一种更快的加法语法。[number-source]_

这里将“向量化”用于数组级表达：用一次数组操作替代 Python 中显式的逐元素循环。它并不意味着 CPU 用一条指令完成整个数组，也不保证某个操作必然并行。

.. note:: 讨论范围

   ufunc 还提供 ``reduce`` 等运算方式。例如
   ``np.add.reduce(a, axis=1)`` 按行归约，结果为 ``[6., 22., 38.]``。
   归约的运算顺序、累加类型与数值误差需要单独讨论，不能把它等同于
   普通逐元素输出。本文先分析通常的 ``np.add(a, b)`` 调用。[reduce]_


2. dtype 与循环选择
-----------------------------------------------------------------------

数组的 dtype 不仅决定字节如何解释，也参与运算实现的选择。``float32`` 与 ``float64`` 的元素大小不同，处理它们的循环不能直接混用。

ufunc 的 ``types`` 属性列出已登记的部分输入、输出类型组合：[ufunc-types]_

.. code-block:: python

   print([s for s in np.add.types if s in {"ff->f", "dd->d"}])
   # ['ff->f', 'dd->d']

``f`` 和 ``d`` 分别表示单精度与双精度浮点类型；箭头左侧是两个输入，右侧是输出。因此 ``dd->d`` 表示两个 ``float64`` 输入对应一个 ``float64`` 输出。

但输入 dtype 不同，并不意味着必须存在与它们逐项匹配的循环。NumPy 可以先确定运算采用的 dtype，再进行必要的转换。``resolve_dtypes`` 可以在不执行加法的情况下观察这种选择：[resolve-dtypes]_

.. code-block:: python

   x = np.array([1, 2], dtype=np.int32)
   y = np.array([0.5, 1.5], dtype=np.float32)

   print(np.add.resolve_dtypes((x.dtype, y.dtype, None)))
   # (dtype('float64'), dtype('float64'), dtype('float64'))

   print(np.add(x, y).dtype)  # float64
   print(x.dtype, y.dtype)    # int32 float32

第三项 ``None`` 表示未指定输出 dtype。返回的三个 ``float64`` 则是这次计算中两个输入和输出采用的类型，不是原数组的 dtype 被修改了。

这一步也不等于必然先创建两个完整的转换副本。转换可以借助迭代缓冲分段进行；类型选择与实际的内存安排是不同层次。[iteration]_

``types`` 适合观察类型组合，不能据此判断本次调用使用了哪种 CPU 指令。它也不是覆盖所有扩展 dtype 的完整清单。[ufunc-reference]_


3. 步长迭代与内层循环
-----------------------------------------------------------------------

确定计算类型后，还需要将多维访问交给执行循环。上一章看到的广播步长，在这里成为循环的输入。

将一维输入改为三行一列，用 ``nditer`` 暴露每段迭代的形状与步长：

.. code-block:: python

   r = np.array([10, 20, 30], dtype=np.float64).reshape(3, 1)
   out = np.empty_like(a)

   with np.nditer(
       [a, r, out],
       flags=["external_loop"],
       op_flags=[["readonly"], ["readonly"], ["writeonly"]],
       order="C",
   ) as it:
       for left, right, dest in it:
           print(left.shape, left.strides, right.strides, dest.strides)
           np.add(left, right, out=dest)

   assert np.array_equal(out, a + r)

本例输出三次：

.. code-block:: text

   (4,) (8,) (0,) (8,)
   (4,) (8,) (0,) (8,)
   (4,) (8,) (0,) (8,)

``external_loop`` 使迭代器每次提供一段一维访问，而不是一个零维元素。本例每段对应一行：左输入与输出前进八字节，右输入前进零字节，连续四次读取同一个行增量。[nditer]_

这里的 Python 循环是观察工具，不是 ``np.add(a, r)`` 的实际调用过程，也不是建议替换原来的数组表达式。迭代段如何划分，会随布局、迭代选项与缓冲方式变化；不能由这个实验推断所有 ufunc 都逐行执行。[iteration]_


源码中的循环接口
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

NumPy v2.3.5 的 ``numpy/_core/src/umath/ufunc_object.c`` 中，``execute_ufunc_loop`` 负责需要迭代器的执行分支。它取得每段的数据指针、步长与长度，再调用内层循环：[ufunc-source]_

.. code-block:: c

   res = strided_loop(context, dataptr, countptr, strides, auxdata);

``dataptr`` 保存各操作数的起始地址，``countptr`` 指向本段元素数，``strides`` 则保存各操作数的字节步长。外层迭代器推进到下一段后，再次调用这个循环。

实际处理 ``float32``、``float64`` 加法的模板位于 ``numpy/_core/src/umath/loops_arithm_fp.dispatch.c.src``。其中标量分支的计算部分为：[arithmetic-source]_

.. code-block:: c

   const @type@ a = *((@type@*)src0);
   const @type@ b = *((@type@*)src1);
   *((@type@*)dst) = a @OP@ b;

这里的 ``@type@``、``@OP@`` 是源码生成时替换的占位符，不是标准 C 语法。对应双精度加法时，它们分别为 ``npy_double`` 和 ``+``。外围循环在每次计算后，按各自步长推进 ``src0``、``src1`` 和 ``dst``。

若第二个输入的步长为零，``src1`` 就保持不变。广播因此不需要在每次加法前重新判断形状：迭代器已经将对应关系表达成地址与步长，内层循环按这份访问描述执行即可。[ufunc-basics]_

对于这里的内置数值循环，不需要逐元素构造 Python 标量、调用 Python 层的加法再赋值。Python 调用的解析与调度成本主要由整次调用承担，循环内部直接处理数值存储。循环依然存在，只是不再由 Python 字节码逐个驱动。[ufunc-basics]_

上述过程是通用分支。源码还提供 ``try_trivial_single_output_loop``，在满足条件时直接执行简单循环，绕过较重的迭代器构造。因此“一次 ufunc 调用”不等于“一定创建一个迭代器”，也不等于“内层函数只调用一次”。[ufunc-source]_

.. admonition:: 配图待制作 F01：广播步长与数值循环
   :class: figure-todo

   **目的：** 将上一篇的零步长与本篇的执行循环对应起来。

   **数据：** 使用 ``a`` 的第一行 ``[0., 1., 2., 3.]``，
   ``r`` 中的 ``10.``，以及输出第一行 ``[10., 11., 12., 13.]``。
   类型均为 ``float64``，每项八字节。

   **内容：** 画三条独立存储带。右输入展示 ``r`` 的三个物理元素，
   但本段只连接第一个。标出本段长度四、左步长八、右步长零、输出步长八；
   用四个读取位置说明右输入地址不变，不画成四份复制的 ``10.``。

   **边界：** 这是标量循环的访问示意，不表示实际执行了哪种 SIMD 指令，
   也不表示所有输入都会被划分成四元素的迭代段。

   **图注：** 广播决定有效步长，内层循环根据地址、步长与元素数执行运算。


4. Python 回调与向量化包装
-----------------------------------------------------------------------

前文的性能解释有一个前提：所选循环直接处理数值，而不是逐元素调用 Python 函数。仅仅把接口改成“可以接收数组”，并不能保证这一点。

给普通加法函数增加调用计数，再用 ``np.vectorize`` 包装：

.. code-block:: python

   calls = 0

   def scalar_add(x, y):
       global calls
       calls += 1
       return x + y

   wrapped = np.vectorize(scalar_add, otypes=[np.float64])
   got = wrapped(a, b)

   print(calls)  # 12
   assert np.array_equal(got, result)

数组有十二个结果，Python 函数仍被调用十二次。这里显式给出 ``otypes``，避免包装器为推断输出类型而额外试调首个元素。[vectorize]_

``np.vectorize`` 提供广播和数组结果组织等便利，但不会把函数体编译为原生数值循环。调用计数明确表明，逐元素执行 Python 函数这项工作没有消失。

甚至“对象属于 ``np.ufunc``”也不足以证明没有 Python 回调。``frompyfunc`` 可以将任意 Python 函数包装成真正的 ufunc：[frompyfunc]_

.. code-block:: python

   calls = 0
   py_add = np.frompyfunc(scalar_add, 2, 1)
   obj_result = py_add(a, b)

   print(isinstance(py_add, np.ufunc))  # True
   print(obj_result.dtype)             # object
   print(calls)                        # 12

   assert np.array_equal(obj_result.astype(np.float64), result)

它支持 ufunc 的调用方式，但每个元素仍由 Python 函数处理，输出也是对象数组。ufunc 是统一的运算接口，性能还取决于具体循环与数据类型。不能只依据名称中的 ``vectorize``，或返回对象的类型，就判断计算已转换为原生数值执行。


5. 执行成本的对照实验
-----------------------------------------------------------------------

调用计数能说明包装器仍在执行 Python 函数，耗时实验则用于观察这种差异的量级。这里对比三种实现：Python 中逐项读写数组、``np.vectorize`` 包装的加法，以及内置 ``np.add``。

为避免把计数器自身的开销带入测试，重新定义不含计数的函数：

.. code-block:: python

   import statistics
   import timeit

   def python_loop(x, y):
       dst = np.empty_like(x)
       for i in range(x.size):
           dst[i] = x[i] + y[i]
       return dst

   def plain_add(x, y):
       return x + y

   vadd = np.vectorize(plain_add, otypes=[np.float64])

   def native_add(x, y):
       return np.add(x, y)

   def measure(func, x, y, number):
       assert np.array_equal(func(x, y), x + y)
       times = timeit.repeat(
           lambda: func(x, y), repeat=5, number=number
       )
       return statistics.median(times) / number * 1e6

输入均为同形、一维、连续的 ``float64`` 数组，提前创建。每种实现都返回新数组，因此结果分配计入耗时；输入生成、导入和正确性检查不计入。测量前调用一次，各重复批次的单次耗时取中位数。[timeit]_

例如测量十万个元素：

.. code-block:: python

   x = np.arange(100_000, dtype=np.float64)
   y = np.full(x.shape, 0.5, dtype=np.float64)

   for func in (python_loop, vadd, native_add):
       print(measure(func, x, y, number=2))  # 单位：微秒 / 调用

配套脚本还测量了较小规模。下面是一次 Linux x86_64 容器中的结果，Python 为 3.13.5，NumPy 为 2.3.5；原始批次数据保存在 ``results.json``。这不是读者本机或不同 NumPy 构建之间的比较。

.. list-table:: 同形 float64 数组加法，单次耗时中位数（微秒）
   :header-rows: 1
   :widths: 20 28 28 24

   * - 元素数
     - Python 循环
     - ``np.vectorize``
     - ``np.add``
   * - 1
     - 0.659
     - 3.294
     - 0.401
   * - 16
     - 3.144
     - 4.473
     - 0.416
   * - 1,024
     - 186.066
     - 114.311
     - 0.661
   * - 100,000
     - 16,470.713
     - 12,386.871
     - 26.124

在这次测量中，``np.add`` 处理一个与十六个元素的时间接近，符合固定调用成本占较大比例的预期；随着元素数增加，它与逐元素 Python 操作的耗时差异扩大。本次 ``np.vectorize`` 在较大规模上快于手写数组索引循环，但仍远慢于 ``np.add``，不能将前者的相对改善理解成函数已被编译。

这里比较的是完整接口成本，而不是单独测量加法指令。Python 循环包含逐元素 ndarray 索引和赋值；包装器还包含对象转换与回调。测试也没有隔离 SIMD、缓存或内存带宽各自的贡献，更不能将这些比值当作固定加速倍数。


6. SIMD 与向量化的层次
-----------------------------------------------------------------------

数组级向量化和 CPU 的 SIMD 是两个层次。前者将循环交给数组操作；后者允许一条机器指令对多个数据执行同一种运算。原生循环即使采用标量指令，也可以避免逐元素解释执行的成本。[simd]_

前面引用的浮点运算模板不只有标量分支，也包含 SIMD 实现。它根据循环长度、内存重叠与步长等条件选择适用路径，其中还专门处理一个输入步长为零的情况。这意味着广播输入可以参与 SIMD 计算，并不必然退回逐项读取。[arithmetic-source]_

但源码存在 SIMD 分支，不代表某次调用必然进入该分支。NumPy 构建可以包含多套目标实现，运行时依据 CPU 能力选择；具体布局和运算还会影响内部路径。[simd]_

``np.show_runtime()`` 可以查看当前环境可用的 SIMD 特性及部分底层库信息，但它不是某次 ``np.add`` 的执行跟踪。看到 AVX2、AVX-512 或其他指令集名称，只能说明环境提供了相关能力，不能由此确定本次运算采用的指令宽度。[runtime]_

同样，向量化不等于自动启用多个线程。NumPy 的普通函数调用通常使用单线程；线性代数后端可能另行使用线程。``show_runtime`` 中显示的 BLAS 线程数，不是 ``np.add`` 的工作线程数。[global-config]_


7. 复合表达式与临时存储
-----------------------------------------------------------------------

将循环交给 NumPy 后，仍有另一类开销：多个数组操作是否重复读写中间结果。

以下表达式包含乘法和加法两次运算：

.. code-block:: python

   tmp = np.multiply(a, 2.0)
   affine = np.add(tmp, 1.0)

   assert np.array_equal(affine, a * 2.0 + 1.0)
   assert not np.shares_memory(tmp, affine)

这里保留了 ``tmp``，加法又创建独立结果。将两行合成 ``a * 2.0 + 1.0``，不会自动把乘法与加法变成同一轮元素循环。[ufunc-reference]_

不过，分配次数与循环次数还要分开。NumPy 的运算符实现包含临时存储复用逻辑，在满足条件时可能复用无其他用途的中间数组。因此不宜仅凭两个运算符，就断言每次一定分配两个全新的缓冲区；即使存储复用成功，也不等于两个循环已经融合。[number-source]_

需要显式复用输出存储时，可以使用 ``out``：

.. code-block:: python

   work = np.empty_like(a)

   np.multiply(a, 2.0, out=work)
   np.add(work, 1.0, out=work)

   assert np.array_equal(work, affine)

这个版本不需要分别保存乘法结果与最终结果，但仍有两次 ufunc 调用：先写入乘积，再读取乘积并写入和。``out`` 控制结果存储，不负责将多个函数编译成一个循环。[add]_

本例输入、工作数组均为 ``float64``，工作数组与输入不重叠，才可以直接按这种方式对照。推广到不同 dtype 或更复杂的重叠布局时，需要重新检查转换规则及数据依赖；提供 ``out`` 也不保证内部完全没有缓冲或临时复制。[ufunc-reference]_

真正的循环融合，是在一次遍历中读入一个元素、完成乘法与加法，再直接写出最终结果。是否需要引入这样的实现，应由实际测量决定，而不是将所有数组表达式一律展开或改写。

.. admonition:: 配图待制作 F02：存储复用与循环融合
   :class: figure-todo

   **目的：** 区分减少缓冲区数量与减少遍历次数。

   **方案一：** 对本文保留 ``tmp`` 的两行代码，画出第一次遍历从 ``a``
   写入 ``tmp``，第二次从 ``tmp`` 写入 ``affine``。

   **方案二：** 对 ``out=work`` 版本，第一次从 ``a`` 写入 ``work``，
   第二次读取并覆盖 ``work``。明确标注“两次遍历，一个工作数组”。

   **方案三：** 单独画概念性的融合循环：一个元素读入后完成乘、加，
   只写出最终值；标注“非普通 NumPy 表达式自动保证的执行方式”。

   **边界：** 不把逻辑读写次数画成确定的 DRAM 访问次数，也不将
   两次浮点运算的循环融合等同于采用一次舍入的融合乘加指令。

   **图注：** 复用输出数组可以减少结果存储；循环融合还改变了中间值的读写方式。


结语
-----------------------------------------------------------------------

广播确定元素对应，dtype 参与循环选择，迭代器将布局整理为地址、步长与长度，具体循环再执行运算。数组级向量化省去的是 Python 层的逐元素组织，而不是运算本身。

分析性能时，还需要区分原生循环与 Python 回调、标量与 SIMD，以及存储复用与循环融合。这些机制可能共同作用，但不是同一项优化。


参考资料
-----------------------------------------------------------------------

.. [ufunc-basics] `NumPy：Universal functions basics <https://numpy.org/doc/2.3/user/basics.ufuncs.html>`_

.. [ufunc-reference] `NumPy：Universal functions <https://numpy.org/doc/2.3/reference/ufuncs.html>`_

.. [number-source] `NumPy v2.3.5：number.c，array_add 与 array_multiply <https://github.com/numpy/numpy/blob/v2.3.5/numpy/_core/src/multiarray/number.c#L215-L246>`_

.. [reduce] `numpy.ufunc.reduce <https://numpy.org/doc/2.3/reference/generated/numpy.ufunc.reduce.html>`_

.. [ufunc-types] `numpy.ufunc.types <https://numpy.org/doc/2.3/reference/generated/numpy.ufunc.types.html>`_

.. [resolve-dtypes] `numpy.ufunc.resolve_dtypes <https://numpy.org/doc/2.3/reference/generated/numpy.ufunc.resolve_dtypes.html>`_

.. [iteration] `NumPy：Iterating over arrays <https://numpy.org/doc/2.3/reference/arrays.nditer.html>`_

.. [nditer] `numpy.nditer <https://numpy.org/doc/2.3/reference/generated/numpy.nditer.html>`_

.. [ufunc-source] `NumPy v2.3.5：ufunc_object.c，execute_ufunc_loop <https://github.com/numpy/numpy/blob/v2.3.5/numpy/_core/src/umath/ufunc_object.c#L971-L1116>`_

.. [arithmetic-source] `NumPy v2.3.5：loops_arithm_fp.dispatch.c.src，浮点四则运算模板 <https://github.com/numpy/numpy/blob/v2.3.5/numpy/_core/src/umath/loops_arithm_fp.dispatch.c.src#L24-L177>`_

.. [vectorize] `numpy.vectorize <https://numpy.org/doc/2.3/reference/generated/numpy.vectorize.html>`_

.. [frompyfunc] `numpy.frompyfunc <https://numpy.org/doc/2.3/reference/generated/numpy.frompyfunc.html>`_

.. [timeit] `Python：timeit <https://docs.python.org/3/library/timeit.html>`_

.. [simd] `NumPy：CPU/SIMD optimizations <https://numpy.org/doc/2.3/reference/simd/index.html>`_

.. [runtime] `numpy.show_runtime <https://numpy.org/doc/2.3/reference/generated/numpy.show_runtime.html>`_

.. [global-config] `NumPy：Global Configuration Options <https://numpy.org/doc/2.3/reference/global_state.html>`_

.. [add] `numpy.add <https://numpy.org/doc/2.3/reference/generated/numpy.add.html>`_