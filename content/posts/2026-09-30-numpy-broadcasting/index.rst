NumPy: 广播机制
=======================================================================

:date: 2026-09-30
:slug: numpy-broadcasting
:tags: numpy, zh
:lang: zh
:draft: false

上一篇讨论了 ``ndarray`` 如何通过形状与步长访问内存。[memory-model]_ 广播沿用这一机制，使形状不同的数组能够参与同一次逐元素运算，而不必先将输入复制成相同大小。

本文从二维数组与一维数组的相加开始，观察广播后的索引如何对应原始数据，再对照迭代器源码，分析零步长的设置与实际运算中的内存开销。

实验使用 NumPy 2.3.5，源码固定到 ``v2.3.5``。以下只讨论普通数值 ``ndarray`` 的逐元素运算。


1. 形状对齐与元素对应
-----------------------------------------------------------------------

对一个三行四列的数组，分别给四列加上不同的数：

.. code-block:: python

   import numpy as np

   a = np.arange(12, dtype=np.int32).reshape(3, 4)
   b = np.array([10, 20, 30, 40], dtype=np.int32)
   c = a + b

   print(c)

.. code-block:: text

   [[10 21 32 43]
    [14 25 36 47]
    [18 29 40 51]]

``a`` 的形状为 ``(3, 4)``，``b`` 为 ``(4,)``。结果中的每个元素满足：

.. math::

   c_{i,j}=a_{i,j}+b_j.

同一个 ``b[j]`` 参与三行的计算，行索引并不影响它的取值。

NumPy 按照从右向左的顺序对齐各轴。两个轴长度相等时保留该长度；其中一个长度为一时，采用另一个轴的长度；缺失的前导轴按长度一处理。其余情况不兼容。[broadcasting]_

因此，``(4,)`` 在本次运算中按 ``(1, 4)`` 对齐，随后与 ``(3, 4)`` 形成共同形状。这个对齐只是运算规则，不会把原数组 ``b.shape`` 改成二维。

可以单独检查形状，不执行加法：[broadcast-shapes]_

.. code-block:: python

   print(np.broadcast_shapes(a.shape, b.shape))  # (3, 4)

右对齐也意味着 NumPy 不会根据“行”或“列”的用途猜测对应关系。一个形状为 ``(3,)`` 的数组不能直接与 ``a`` 相加，因为尾轴的 ``3`` 与 ``4`` 不兼容。若要分别给三行加上不同的值，应显式保留列轴：

.. code-block:: python

   per_row = np.array([100, 200, 300], dtype=np.int32)

   print(per_row[:, None].shape)                         # (3, 1)
   print(np.broadcast_shapes(a.shape, per_row[:, None].shape))  # (3, 4)

这不是将三元素数组补成四元素数组，而是声明它的三个值对应哪一个轴。广播也不是循环填充：长度为二的轴，不能仅因四是二的倍数就广播为长度四。


2. 零步长与数据复用
-----------------------------------------------------------------------

形状对齐规定了哪些元素参与运算，但还没有说明它们如何存储。若真把 ``b`` 复制成三行，列步长应当仍为四字节，行步长则是十六字节。

用 ``np.broadcast_to`` 显式构造相应的广播视图：[broadcast-to]_

.. code-block:: python

   bb = np.broadcast_to(b, a.shape)

   print(bb.shape)                        # (3, 4)
   print(bb.strides)                      # (0, 4)
   print(bb.ctypes.data == b.ctypes.data)  # True
   assert np.shares_memory(b, bb)

行步长不是十六，而是零。代入上一篇的寻址公式：

.. math::

   \operatorname{address}_{bb}(i,j)
   =\operatorname{data}_b+i\times 0+j\times 4
   =\operatorname{data}_b+4j.

行索引变化时，地址不变。所谓三行数据，实际上是三个逻辑行访问同一份四元素存储。

以第三列为例，三行的字节偏移均为八。仍可以用裸地址读取验证：

.. code-block:: python

   import ctypes

   offsets = [i * bb.strides[0] + 2 * bb.strides[1] for i in range(3)]
   print(offsets)  # [8, 8, 8]

   address = bb.ctypes.data + offsets[2]
   value = ctypes.c_int32.from_address(address).value
   assert value == bb[2, 2] == 30

这里保留了原数组，只读取已知有效的位置。``ctypes`` 不检查数组边界，地址整数本身也不会维持数组的生命周期。[ctypes]_

``bb`` 的形状与步长属于这个新视图，原来的 ``b`` 仍是一维数组，步长仍为 ``(4,)``。广播没有修改原数组的布局，也没有复制它的元素；新视图及其元数据仍然需要内存。


逻辑大小与存储大小
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

此时查看 ``nbytes``，会得到两个不同的数：

.. code-block:: python

   print(b.nbytes)   # 16
   print(bb.nbytes)  # 48

``nbytes`` 按逻辑元素数乘以元素大小计算，不统计不同索引实际占用了多少个互异的内存位置。[nbytes]_

``bb`` 有十二个逻辑元素，所以报告四十八字节；这些索引却只访问 ``b`` 中的四个整数。因此，不能用 ``bb.nbytes - b.nbytes`` 判断广播额外分配了多少内存。


3. 两侧广播的地址映射
-----------------------------------------------------------------------

广播不必由一个较小数组单方面匹配另一个较大数组。两个输入可以分别沿不同的轴复用数据，共同形成比两者都大的结果。

继续使用 ``b``，再引入一个三行一列的数组：

.. code-block:: python

   r = np.arange(3, dtype=np.int32).reshape(3, 1)
   rr = np.broadcast_to(r, (3, 4))

   print(rr.strides)  # (4, 0)
   print(bb.strides)  # (0, 4)
   print(r + b)

.. code-block:: text

   [[10 20 30 40]
    [11 21 31 41]
    [12 22 32 42]]

``r`` 沿列方向复用，所以列步长为零；``b`` 沿行方向复用，所以行步长为零。对于结果位置 ``(i, j)``，两个输入地址分别为：

.. math::

   \begin{aligned}
   \operatorname{address}_{rr}(i,j)&=\operatorname{data}_r+4i,\\
   \operatorname{address}_{bb}(i,j)&=\operatorname{data}_b+4j.
   \end{aligned}

两个操作数使用同一组逻辑索引，但各自决定哪些索引影响地址。结果 ``(2, 1)`` 从 ``r`` 的偏移八字节处读取 ``2``，从 ``b`` 的偏移四字节处读取 ``20``，相加得到 ``22``。

长度为一的轴只有索引零。沿这个轴广播，意味着结果坐标无论取何值，输入坐标都保持为零；在地址计算中，这恰好可以由零步长表示。缺失的前导轴采用同样的处理。

因此，广播后的形状描述共同的迭代范围，各操作数的有效步长则描述结果索引如何映射到自身数据。右对齐是 NumPy 规定的对应规则，零步长是实现数据复用的一种直接表示。[ufunc-basics]_

.. note:: 配图待制作 F01：两侧广播的地址映射

   **目的：** 对比共同的逻辑形状与两份实际输入存储。

   **数据：** ``r`` 的三个整数为 ``0, 1, 2``，形状 ``(3, 1)``；
   ``b`` 的四个整数为 ``10, 20, 30, 40``，形状 ``(4,)``；
   两者均为 ``int32``。

   **布局：** 展示 ``rr``、``bb`` 的 ``3×4`` 逻辑网格，并分别标注
   步长 ``(4, 0)`` 与 ``(0, 4)``。实际存储只画三格和四格，不能画成
   两份各有十二个元素的独立缓冲区。

   **重点：** 高亮逻辑位置 ``(2, 1)``，将两个读取位置分别连到
   ``r`` 的第三个元素和 ``b`` 的第二个元素，标出偏移 ``8``、``4``
   及结果 ``22``。用少量代表性连线表达复用，不必连接所有索引。

   **图注：** 两个广播视图具有相同形状，但使用不同的零步长轴；
   它们共享逻辑索引，而不共享彼此的数据。


4. 广播视图的只读约束
-----------------------------------------------------------------------

零步长使同一数组内部的多个逻辑位置指向同一地址。在 ``bb`` 中，``(0, 0)``、``(1, 0)`` 和 ``(2, 0)`` 并不是三个独立的存储位置。

如果允许像普通独立数组那样赋值，修改一个位置便会同时改变其他位置的读取结果。``np.broadcast_to`` 因此返回只读视图，尝试写入会抛出异常。[broadcast-to]_

.. code-block:: python

   print(bb.flags.writeable)  # False

   try:
       bb[0, 0] = 99
   except ValueError as exc:
       print(exc)  # assignment destination is read-only

只读约束作用于这个视图，不会冻结底层数据。原数组 ``b`` 仍然可写，其修改会反映到所有共享该位置的逻辑元素：

.. code-block:: python

   b[0] = 99
   print(bb[:, 0])  # [99 99 99]
   b[0] = 10       # 恢复前面的实验数据

这里没有发生三次赋值，只改动了 ``b`` 中的一个整数。若需要一个可独立修改的 ``(3, 4)`` 数组，应使用 ``bb.copy()``，而不是尝试强行开启广播视图的写权限。


5. 迭代器中的步长设置
-----------------------------------------------------------------------

前面的实验可以在源码中找到对应。NumPy v2.3.5 的 ``numpy/lib/_stride_tricks_impl.py`` 中，``broadcast_to`` 调用内部函数 ``_broadcast_to``，后者通过 ``np.nditer`` 创建迭代器，再取出其数组视图。[broadcast-source]_

以下摘录构造与取视图的部分；此处 ``extras`` 是空列表：

.. code-block:: python

   it = np.nditer(
       (array,), flags=['multi_index', 'refs_ok', 'zerosize_ok'] + extras,
       op_flags=['readonly'], itershape=shape, order='C')
   with it:
       broadcast = it.itviews[0]

``itershape`` 指定目标迭代形状，``op_flags`` 指定输入只读，``multi_index`` 要求保留多维索引。``itviews`` 提供与迭代器访问布局对应的数组视图。[nditer]_

这里没有遍历目标形状的所有元素，也没有逐行复制。构造迭代器时已经建立了访问描述，返回视图即可暴露这套描述。

继续进入 ``numpy/_core/src/multiarray/nditer_constr.c``。``npyiter_fill_axisdata`` 先确定广播形状，再填写各轴上每个操作数的步长。在未指定自定义轴映射的分支中，包含以下判断，省略分支内的后续检查：[iterator-source]_

.. code-block:: c

   else if (idim >= ondim ||
                   PyArray_DIM(op_cur, ondim-idim-1) == 1) {
       strides[iop] = 0;
       /* 后续检查省略。 */
   }

``idim`` 从最右侧的轴开始计数，``ondim`` 是当前操作数的维数。条件分别对应两种情况：当前操作数没有这个前导轴，或者对应轴长度为一。两者都使用零步长；对应的非广播分支则读取原数组的轴步长。

需要注意，``strides[iop]`` 是迭代器当前轴上，第 ``iop`` 个操作数的有效步长，不是直接改写该数组的 ``strides`` 属性。一次迭代可以为多个输入分别保存访问方式，而不改变任何原数组的形状与步长。

这也说明，``np.broadcast_to`` 只是将广播访问方式显式呈现为一个视图；不能据此认为 ``a + b`` 必须先调用两次 ``broadcast_to``。逐元素加法由 ufunc 执行，其迭代机制同样能够沿广播轴保持输入地址不变。[ufunc-basics]_


6. 结果存储与计算开销
-----------------------------------------------------------------------

输入不需要扩张，并不意味着加法结果也只是视图。回到最初的 ``c = a + b``：

.. code-block:: python

   print(c.nbytes)  # 48
   assert not np.shares_memory(c, a)
   assert not np.shares_memory(c, b)

结果中的十二个整数需要独立保存。对于这里的普通数组，``np.add`` 未提供 ``out`` 时会分配结果数组；提供 ``out`` 则写入指定的输出存储。[add]_

.. code-block:: python

   out = np.empty_like(a)
   result = np.add(a, b, out=out)

   assert result is out
   assert np.array_equal(out, c)

``out`` 允许复用事先分配的结果空间，不会使十二个结果只占四个元素的位置。也不能将它理解为“整个运算保证不使用临时内存”：类型转换、对齐或输入输出重叠等情况，仍可能需要缓冲或临时副本。[ufunc-basics]_ [ufunc-reference]_

更明显的例子是两个一万元素的 ``float64`` 向量。将它们分别表示为 ``(10000, 1)`` 和 ``(1, 10000)``，两份输入数据合计只有 160,000 字节；但相加所得形状是 ``(10000, 10000)``，完整结果需要：

.. math::

   10000\times 10000\times 8
   =800{,}000{,}000\ \text{bytes}.

即十进制的 800 MB，尚未计入对象和其他开销。这个大小可以只通过形状与 dtype 计算，不必真的分配数组。

广播省去的是为重复使用输入而制造的副本，不会省去结果的元素数。只要要求完整保留这次加法的稠密结果，就仍需与结果规模成正比的存储和写入工作。较大的广播表达式因此可能产生较大的结果或中间数组，即便每份输入本身都很小。[broadcasting]_


结语
-----------------------------------------------------------------------

广播规定不同形状的数组如何对应元素；零步长使某些结果坐标不再影响输入地址，从而复用原始数据。输入的存储规模与输出的逻辑规模，应分别考虑。

广播与向量化也不是同一概念。前者解决元素对应，后者通常指通过数组级操作替代 Python 中的逐元素循环。同形数组的相加同样是向量化操作，却不需要扩张任何轴。至于底层循环是否使用 SIMD，则属于执行实现，不能仅凭发生了广播作出判断。[ufunc-basics]_ [simd]_


参考资料
-----------------------------------------------------------------------

.. [memory-model] `NumPy: ndarray 的内存模型 <https://6ixgodd.github.io/posts/numpy-ndarray-memory-model/>`_

.. [broadcasting] `NumPy：Broadcasting <https://numpy.org/doc/2.3/user/basics.broadcasting.html>`_

.. [broadcast-shapes] `numpy.broadcast_shapes <https://numpy.org/doc/2.3/reference/generated/numpy.broadcast_shapes.html>`_

.. [broadcast-to] `numpy.broadcast_to <https://numpy.org/doc/2.3/reference/generated/numpy.broadcast_to.html>`_

.. [ctypes] `numpy.ndarray.ctypes <https://numpy.org/doc/2.3/reference/generated/numpy.ndarray.ctypes.html>`_

.. [nbytes] `numpy.ndarray.nbytes <https://numpy.org/doc/2.3/reference/generated/numpy.ndarray.nbytes.html>`_

.. [broadcast-source] `NumPy v2.3.5：_broadcast_to <https://github.com/numpy/numpy/blob/v2.3.5/numpy/lib/_stride_tricks_impl.py#L340-L360>`_

.. [nditer] `numpy.nditer <https://numpy.org/doc/2.3/reference/generated/numpy.nditer.html>`_

.. [iterator-source] `NumPy v2.3.5：npyiter_fill_axisdata <https://github.com/numpy/numpy/blob/v2.3.5/numpy/_core/src/multiarray/nditer_constr.c#L1474-L1511>`_

.. [add] `numpy.add <https://numpy.org/doc/2.3/reference/generated/numpy.add.html>`_

.. [ufunc-basics] `NumPy：Universal functions basics <https://numpy.org/doc/2.3/user/basics.ufuncs.html>`_

.. [ufunc-reference] `NumPy：Universal functions <https://numpy.org/doc/2.3/reference/ufuncs.html>`_

.. [simd] `NumPy：CPU/SIMD optimizations <https://numpy.org/doc/2.3/reference/simd/index.html>`_