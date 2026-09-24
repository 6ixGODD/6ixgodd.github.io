NumPy: ndarray 的内存模型
==================================================

:date: 2026-09-24
:slug: numpy-ndarray-memory-model
:tags: numpy, zh
:lang: zh
:draft: false

NumPy 为什么快？

一种常见的解释是：它将逐元素运算交给编译后的 C 代码执行，减少了 Python 循环中解释执行和操作 Python 对象的开销。

不过，执行语言并不是性能的唯一因素，数据的组织和访问方式同样重要。本文以 ``a[2, 1]`` 的访问过程为线索，分析 ``ndarray`` 的内存布局，以及多维索引与元素地址之间的关系。


1. 从索引到内存
--------------------------------------------------

以下是一个三行四列的整数数组：

.. code-block:: python

   import numpy as np

   a = np.array(
       [
           [0, 1, 2, 3],
           [4, 5, 6, 7],
           [8, 9, 10, 11],
       ],
       dtype=np.int32,
   )

   print(a.shape)    # (3, 4)
   print(a.strides)  # (16, 4)

``shape`` 表示各维度的长度，``strides`` 则记录沿各轴移动一个索引位置时，元素地址的字节增量。

在这个数组中，每个 ``int32`` 占四字节，元素按行连续存储。列索引增加一，地址增加四字节；行索引增加一，地址增加一行的大小，即十六字节。因此，``a[2, 1]`` 相对于首元素的偏移为：

.. math::

   2 \times 16 + 1 \times 4 = 36\ \text{bytes}

通过 ``a.ctypes.data`` 取得数据地址，可以使用标准库 ``ctypes`` 直接读取这个位置，验证上述计算：

.. code-block:: python

   import ctypes

   offset = 2 * a.strides[0] + 1 * a.strides[1]
   address = a.ctypes.data + offset
   value = ctypes.c_int32.from_address(address).value

   print(value)  # 9
   assert value == a[2, 1]

读取结果与 NumPy 一致。对于二维数组，合法的非负整数索引可以按以下公式转换为地址：

.. math::

   \operatorname{address}(i, j)
   =
   \operatorname{data}
   + i \times \operatorname{strides}[0]
   + j \times \operatorname{strides}[1]

``shape`` 用于限定索引范围，并不直接参与地址计算。步长已经以字节为单位，也不需要再乘元素大小。

这里的 ``(16, 4)`` 由当前布局决定，并非二维数组的固定形式。寻址公式允许其他步长，这一点使 NumPy 能够用同一份数据表示不同的数组布局。

.. note::

   本文保留了被访问的数组，并且只读取已知有效的地址。
   ``ctypes`` 不检查数组索引是否越界；
   ``a.ctypes.data`` 返回的地址整数也不会维持数组的生命周期。


2. 视图与步长
--------------------------------------------------

二维转置满足 ``b[j, i] == a[i, j]``。按照上一节的寻址公式，只要 ``b`` 沿用 ``a`` 的数据地址，并交换两个轴的步长，就可以维持这一对应关系，无须重新排列元素。

实际结果如下：

.. code-block:: python

   b = a.T

   print(b.shape)                         # (4, 3)
   print(b.strides)                       # (4, 16)
   print(b.ctypes.data == a.ctypes.data)  # True

``b[1, 2]`` 的偏移变成 ``1 * 4 + 2 * 16``，仍为三十六字节，与 ``a[2, 1]`` 指向同一个位置。

这里需要区分数组对象与元素数据。``b`` 是新的数组对象，具有自己的形状和步长，但与 ``a`` 共享元素数据。这类数组称为视图（view）。

NumPy v2.5.2 的 ``numpy/_core/src/multiarray/shape.c`` 中，``PyArray_Transpose`` 在创建返回数组时复用了 ``PyArray_DATA(ap)``，随后根据轴顺序填写维度和步长：

.. code-block:: c

   for (i = 0; i < n; i++) {
       PyArray_DIMS(ret)[i] = PyArray_DIMS(ap)[permutation[i]];
       PyArray_STRIDES(ret)[i] = PyArray_STRIDES(ap)[permutation[i]];
   }

其中，``ap`` 是原数组，``ret`` 是返回数组。二维转置的 ``permutation`` 为 ``[1, 0]``，因此两个轴的长度和步长分别交换。循环遍历的是轴，而非数组中的元素。


切片的地址变化
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

基本切片同样可以通过修改步长和数据起点实现。

例如，``a[:, ::2]`` 保留每隔一列的元素。新数组的列索引增加一，对应原数组的列索引增加二，因此列步长应由四字节变为八字节：

.. code-block:: python

   c = a[:, ::2]

   print(c.shape)    # (3, 2)
   print(c.strides)  # (16, 8)

这些元素仍然位于原来的位置，并没有被复制到一块新的连续存储中。切片后相邻元素之间可以存在间隔。

反向切片还涉及数据起点的变化。``a[::-1]`` 的首元素对应 ``a[2, 0]``，所以数据指针需要增加三十二字节；行索引递增时，访问顺序与原数组相反，行步长相应变为负数：

.. code-block:: python

   d = a[::-1]

   print(d.shape)                         # (3, 4)
   print(d.strides)                       # (-16, 4)
   print(d.ctypes.data - a.ctypes.data)   # 32

原有的寻址公式仍然适用。例如：

.. math::

   \operatorname{address}_d(2, 1)
   =
   (\operatorname{data}_a + 32)
   + 2 \times (-16)
   + 1 \times 4
   =
   \operatorname{data}_a + 4

因此，``d[2, 1]`` 对应 ``a[0, 1]``，其值为 ``1``。

反向切片说明，数据指针应当理解为当前非空数组的逻辑首元素地址，而不是底层内存分配的起始地址。``a`` 与 ``d`` 的数据指针虽然不同，访问的内存仍然重叠。判断是否共享内存，不能只比较这两个地址：

.. code-block:: python

   assert np.shares_memory(a, b)
   assert np.shares_memory(a, c)
   assert np.shares_memory(a, d)

同一块存储由此产生了转置、间隔切片和反向切片三种视图。它们的差异都体现在索引与地址的映射中。

.. note::

   连续性与共享内存是不同的属性。本例中，``a`` 是 C-contiguous，
   ``b`` 是 F-contiguous，而 ``c``、``d`` 两者都不是。
   转置后的数组不再按行连续，并不意味着它在任何顺序下都不连续。


3. dtype 与元素布局
--------------------------------------------------

前文的实验在计算地址后，使用 ``ctypes.c_int32`` 读取数据。这个类型并不是由地址决定的：同一位置的字节，采用不同类型解释，可以得到不同的值。

.. code-block:: python

   x = np.array([1.0], dtype=np.float32)
   y = x.view(np.uint32)
   z = x.astype(np.uint32)

   print(x[0])  # 1.0
   print(y[0])  # 1065353216
   print(z[0])  # 1

   assert np.shares_memory(x, y)
   assert not np.shares_memory(x, z)

``x`` 与 ``y`` 共享四个字节，分别按 ``float32`` 和 ``uint32`` 解释。``view`` 没有进行数值转换；``1065353216`` 是原有浮点表示被当作无符号整数读取的结果。

``astype`` 则将数值 ``1.0`` 转换成整数 ``1``，本例中为转换结果分配了独立存储。重新解释已有字节与转换数值，是两种不同的操作。

dtype 描述的就是元素的解释规则，包括类型、大小、字节序，以及结构化类型的字段信息。数组的形状和步长负责元素之间的布局，dtype 还可以描述一个元素内部的布局。


结构化 dtype
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

下面的数组将编号与分数保存在同一个元素中：

.. code-block:: python

   dt = np.dtype([
       ("id", np.int32),
       ("score", np.float64),
   ])
   records = np.array([(1, 3.5), (2, 7.0)], dtype=dt)

   print(dt.itemsize)            # 12
   print(dt.fields["score"][1])  # 4
   print(records.strides)        # (12,)

每条记录占十二字节，``id`` 位于起点，``score`` 的字段偏移为四字节。访问 ``records[1]["score"]`` 时，先根据数组步长定位第二条记录，再加上字段偏移：

.. math::

   \operatorname{address}
   =
   \operatorname{data} + 1 \times 12 + 4

将 ``score`` 字段单独取出，可以观察到元素大小与步长的区别：

.. code-block:: python

   scores = records["score"]

   print(scores.itemsize)                           # 8
   print(scores.strides)                            # (12,)
   print(scores.ctypes.data - records.ctypes.data)  # 4

``scores`` 是共享原数据的字段视图。每个分数占八字节，相邻分数却相距十二字节，因为记录中还保存了编号。元素大小描述元素自身的存储长度，步长描述沿某个轴移动时的地址增量，两者不能等同。

上述 dtype 默认采用 ``align=False``，字段之间没有对齐填充，因此每条记录为十二字节。与 C 代码交换数据时，应核对字段偏移和结构体大小，不能直接假定它与同字段的 C 结构体布局一致。


C API 中的元素读取
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

C 扩展可以自行计算元素地址，再将类型相关的读取交给 NumPy。下面的片段假定 NumPy C API 已初始化，``arr`` 为二维数组，``i``、``j`` 为经过边界检查的非负 ``npy_intp`` 索引：

.. code-block:: c

   char *address = PyArray_BYTES(arr)
                 + i * PyArray_STRIDE(arr, 0)
                 + j * PyArray_STRIDE(arr, 1);

   return PyArray_GETITEM(arr, address);

地址计算与前文相同，但不再将地址处的数据固定解释为 ``int32``。``PyArray_GETITEM`` 根据数组的 dtype 读取该位置并返回 Python 对象。

NumPy v2.5.2 在 ``numpy/_core/include/numpy/ndarrayobject.h`` 中为下游扩展提供的实现如下：

.. code-block:: c

   static inline PyObject *
   PyArray_GETITEM(const PyArrayObject *arr, const char *itemptr)
   {
       return PyDataType_GetArrFuncs(PyArray_DESCR(arr))->getitem(
           (void *)itemptr, (PyArrayObject *)arr);
   }

``PyArray_DESCR`` 取得 dtype descriptor，``PyDataType_GetArrFuncs`` 取得对应的操作函数表，最终调用其中的 ``getitem``。类型差异由 dtype 对应的读取函数处理，外部代码不必为每种类型重复实现寻址逻辑。

这个接口的取值语义对应 ``ndarray.item()``，并不完整复现普通索引。例如，``float32`` 数组的普通索引返回 NumPy 的 ``float32`` 标量，而 ``item()`` 返回 Python 的 ``float``。

.. note:: 关于 dtype descriptor 的访问

   NumPy 2.0 调整了 ``PyArray_Descr`` 的结构，部分字段应改用访问接口，
   而不是沿用旧代码中的直接字段访问。

   例如，读取元素大小时，已有数组可使用 ``PyArray_ITEMSIZE(arr)``；
   只有 descriptor 时，可使用 ``PyDataType_ELSIZE(descr)``，
   不宜继续依赖 ``descr->elsize``。

   NumPy 内部实现与下游扩展的兼容性要求不同。
   阅读源码时可以研究结构体布局，编写扩展时则应优先使用公开接口。


4. 内存所有权与生命周期
--------------------------------------------------

视图共享已有存储，也因此依赖这块存储的有效性。除了数据地址，NumPy 还需要保留相应的对象引用，避免数据在视图仍被使用时释放。

可以通过 ``OWNDATA`` 和 ``base`` 观察这种关系：

.. code-block:: python

   owner = np.arange(6, dtype=np.int32)
   view = owner[::2]

   print(owner.flags.owndata)  # True
   print(owner.base is None)   # True
   print(view.flags.owndata)   # False
   print(view.base is owner)   # True

``owner`` 拥有数据内存，``view`` 不拥有数据，而是通过 ``base`` 引用 ``owner``。

删除变量 ``owner`` 后，视图仍然有效：

.. code-block:: python

   del owner

   print(view)       # [0 2 4]
   print(view.base)  # [0 1 2 3 4 5]

``del owner`` 解除的是变量名与对象的绑定。原数组仍被 ``view.base`` 引用，因此对象及其数据继续存活；这里既没有复制数据，也没有转移数据所有权。

这与第一节保存裸地址整数的做法不同。地址整数不持有数组引用，而视图通过对象引用维持共享存储的生命周期。

``base`` 不一定是创建当前视图时的直接来源。NumPy 可以沿已有的 ``base`` 链追溯到底层对象，因此不能用它还原数组经历过的切片或转置操作。

这一机制也意味着，小切片可能使整块原始数据继续占用内存。需要解除这种依赖时，可以复制切片；新副本拥有独立存储，原始数据则在不再被其他对象引用后才具备释放条件。


结语
--------------------------------------------------

``ndarray`` 将元素数据与数组描述分开管理。数据指针和步长用于寻址，dtype 用于解释元素，形状与引用关系分别约束索引范围和存储的生命周期。

这种设计允许多个数组对象以不同布局访问同一份数据。转置和基本切片因此可以避免元素复制：它们的效率不仅来自执行代码的语言，也来自内存模型本身。


参考资料
--------------------------------------------------

* `What is NumPy? <https://numpy.org/doc/stable/user/whatisnumpy.html>`_

* `The N-dimensional array <https://numpy.org/doc/stable/reference/arrays.ndarray.html>`_

* `numpy.ndarray.ctypes <https://numpy.org/doc/stable/reference/generated/numpy.ndarray.ctypes.html>`_

* `Copies and views <https://numpy.org/doc/stable/user/basics.copies.html>`_

* `NumPy v2.5.2：shape.c <https://github.com/numpy/numpy/blob/v2.5.2/numpy/_core/src/multiarray/shape.c>`_

* `numpy.shares_memory <https://numpy.org/doc/stable/reference/generated/numpy.shares_memory.html>`_

* `numpy.ndarray.view <https://numpy.org/doc/stable/reference/generated/numpy.ndarray.view.html>`_

* `numpy.ndarray.astype <https://numpy.org/doc/stable/reference/generated/numpy.ndarray.astype.html>`_

* `Data type objects <https://numpy.org/doc/stable/reference/arrays.dtypes.html>`_

* `Structured arrays <https://numpy.org/doc/stable/user/basics.rec.html>`_

* `Array API <https://numpy.org/doc/stable/reference/c-api/array.html>`_

* `NumPy v2.5.2：ndarrayobject.h <https://github.com/numpy/numpy/blob/v2.5.2/numpy/_core/include/numpy/ndarrayobject.h>`_

* `NumPy 2.0 migration guide <https://numpy.org/doc/stable/numpy_2_0_migration_guide.html>`_

* `Python：The del statement <https://docs.python.org/3/reference/simple_stmts.html#the-del-statement>`_

* `Indexing on ndarrays <https://numpy.org/doc/stable/user/basics.indexing.html>`_
