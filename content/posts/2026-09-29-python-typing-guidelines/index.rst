Python: 类型系统的使用约定
==================================================

:date: 2026-09-29
:slug: python-typing-guidelines
:tags: python, typing, zh
:lang: zh
:draft: true

Python 是动态类型语言。类型注解没有改变这一点。

本文记录笔者在 Python 项目中使用静态类型的一组约定。目的不是在运行时重新实现一套类型检查，而是在编码阶段尽可能明确接口、数据结构和类型之间的关系，并让静态检查器能够验证这些约定。

本文以 Python 3.14 和 MyPy 为基准。


1. 静态检查必须通过
--------------------------------------------------

类型注解如果只用于补全，而不能通过静态检查，其价值有限。

项目中的 MyPy 检查应当保持通过。新代码原则上不引入未解决的类型错误，也不通过大量 ``# type: ignore`` 将问题隐藏起来。

一个较严格的基础配置可以从下面开始：

.. code-block:: toml

   [tool.mypy]
   python_version = "3.14"

   strict = true

   disallow_any_explicit = true
   disallow_any_expr = true
   disallow_any_unimported = true

   show_error_codes = true
   pretty = true

其中 ``strict`` 打开一组严格检查，但对于 ``Any`` 的限制还可以进一步收紧。对于必须接入缺少类型信息的第三方库等边界场景，可以针对具体模块单独放宽，而不是降低整个项目的检查级别。


Any
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``Any`` 会切断类型检查。

一个值进入 ``Any`` 后，静态检查器基本不再约束它的属性访问、参数传递和返回值，因此 ``Any`` 很容易沿调用链向外扩散。

因此，具体类型优先于 ``Any``：

.. code-block:: python

   # 不推荐

   def normalize(data: Any) -> Any:
       ...


   # 更明确

   def normalize(data: Sequence[float]) -> list[float]:
       ...

这并不意味着 ``Any`` 永远不能出现。某些动态边界、本身没有类型信息的第三方接口，或者确实需要接受任意 Python 对象的底层抽象，仍可能需要它。

问题不在于某一行代码是否出现 ``Any``，而在于这种动态性是否被限制在明确的边界内。


type: ignore
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``# type: ignore`` 只用于已经理解其原因、同时现有类型系统无法正确表达的情况。

应尽量指定错误代码：

.. code-block:: python

   value = library.magic()  # type: ignore[attr-defined]

而不是：

.. code-block:: python

   value = library.magic()  # type: ignore

如果一个 ``ignore`` 无法说明自己忽略的是什么，它通常已经过于宽泛。

处理类型错误时，大致按照下面的顺序：

1. 修正错误的类型声明；
2. 补充缺失的泛型、Protocol 或 overload；
3. 在确定运行时类型但静态检查器无法推导时，使用窄范围的 ``cast``；
4. 最后才使用 ``# type: ignore[code]``。

静态检查的目标不是消灭所有特殊情况，而是让特殊情况保持可见。


2. 类型应该表达关系
--------------------------------------------------

类型注解不只需要说明“这里允许哪些类型”，还应尽可能表达输入与输出之间的关系。

例如：

.. code-block:: python

   def duplicate(value: int | float) -> list[int] | list[float]:
       return [value, value]

这份声明允许以下错误组合：

.. code-block:: text

   int   -> list[float]
   float -> list[int]

虽然实现没有这样做，但函数签名没有表达这种约束。

如果返回值与输入类型保持一致，应使用类型参数：

.. code-block:: python

   def duplicate[T](value: T) -> list[T]:
       return [value, value]

现在类型关系成为函数签名的一部分：

.. code-block:: text

   T -> list[T]


受约束的类型参数
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

如果函数只接受有限的几种类型，同时要求参数与返回值保持同一种类型，可以进一步限制类型参数：

.. code-block:: python

   def concat[T: (str, bytes)](left: T, right: T) -> T:
       return left + right

这里的重点不是 ``str | bytes``，而是三个位置必须由同一个 ``T`` 解释。

因此：

.. code-block:: python

   concat("a", "b")     # str
   concat(b"a", b"b")   # bytes

而混用两种类型应当在静态检查阶段被拒绝。


overload
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

并不是所有对应关系都适合用一个类型变量表示。

当返回类型取决于某个离散的参数组合时，可以使用 ``overload``：

.. code-block:: python

   from typing import Literal, overload


   @overload
   def read(data: bytes, decode: Literal[True]) -> str: ...


   @overload
   def read(data: bytes, decode: Literal[False]) -> bytes: ...


   def read(data: bytes, decode: bool) -> str | bytes:
       if decode:
           return data.decode()
       return data

``overload`` 描述的是若干合法调用形式，真正的运行时实现仍然只有最后一个函数。

在能够使用类型参数完整表达关系时，不需要为了“更严格”而堆叠 ``overload``。只有关系无法由一个泛型签名准确表示时，再使用重载。


3. Callable 与参数转发
--------------------------------------------------

函数本身也有类型。

下面的注解只说明 ``callback`` 可以被调用并返回整数：

.. code-block:: python

   from collections.abc import Callable

   callback: Callable[..., int]

``...`` 放弃了对参数列表的描述。对于真正接受任意调用形式的接口，这没有问题；但如果一个高阶函数只是把原函数的参数继续向下传递，就没有必要丢失这些信息。

Python 的 ``ParamSpec`` 可以保留完整调用签名：

.. code-block:: python

   from collections.abc import Callable
   from functools import wraps


   def traced[R, **P](func: Callable[P, R]) -> Callable[P, R]:
       @wraps(func)
       def wrapper(*args: P.args, **kwargs: P.kwargs) -> R:
           print(func.__name__)
           return func(*args, **kwargs)

       return wrapper

如果输入函数是：

.. code-block:: python

   def add(x: int, y: int) -> int:
       return x + y

经过 ``traced`` 后，它的静态签名仍然是：

.. code-block:: text

   (int, int) -> int

而不是退化为：

.. code-block:: text

   (...) -> Any

Python 3.14 的类型参数语法同样可以直接声明带 ``ParamSpec`` 的类型别名：

.. code-block:: python

   type IntCallable[**P] = Callable[P, int]


动态参数
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``*args`` 与 ``**kwargs`` 本身没有问题。装饰器、代理和参数转发天然需要它们。

但如果一个普通接口可以通过明确的形参表达，就不应因为省事而改成动态解包：

.. code-block:: python

   # 不推荐

   def create_user(*args: object, **kwargs: object) -> User:
       ...


   # 更明确

   def create_user(name: str, age: int, active: bool = True) -> User:
       ...

动态接口应该源于接口本身的需求，而不是为了少写几个参数。


4. 优先使用结构类型
--------------------------------------------------

Python 本身大量依赖鸭子类型。静态类型设计没有必要破坏这一点。

如果一段代码只关心对象是否具有某些行为，应优先使用 ``Protocol``：

.. code-block:: python

   from typing import Protocol


   class Reader(Protocol):
       def read(self, size: int = -1) -> bytes:
           ...


   def consume(reader: Reader) -> bytes:
       return reader.read()

任何具有兼容 ``read`` 方法的对象都可以传给 ``consume``，无需继承 ``Reader``。

这与 Python 原本的接口习惯一致：调用方依赖行为，而不是要求对象声明自己属于某个继承体系。


什么时候使用 ABC
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``Protocol`` 并不取代 ``ABC``。

如果某个组件体系本身具有明确的名义关系，需要所有实现显式继承同一个基类，或者基类还承担共享实现、生命周期管理等职责，那么 ``ABC`` 更合适。

例如，一个框架内部定义的存储驱动：

.. code-block:: python

   from abc import ABC, abstractmethod


   class Storage(ABC):
       @abstractmethod
       def open(self) -> None:
           ...

       @abstractmethod
       def close(self) -> None:
           ...

这里的继承关系本身属于框架设计的一部分。

简单地说：

.. code-block:: text

   只需要“具备这些行为”        -> Protocol
   需要“属于这个抽象体系”      -> ABC

默认使用结构类型，在确实需要名义约束时再引入继承。


5. 使用已有的类型语义
--------------------------------------------------

类型系统中已经存在用于表达常见语义的专门构造。能够准确表达时，不需要重新用 ``TypeVar`` 或宽泛的类型拼装一遍。


Self
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

对于返回当前类型的方法，尤其是 alternative constructor，应使用 ``Self``：

.. code-block:: python

   from typing import Self


   class User:
       def __init__(self, name: str) -> None:
           self.name = name

       @classmethod
       def from_string(cls, value: str) -> Self:
           return cls(value.strip())

这样，子类调用 ``from_string`` 时，返回类型仍然能够保持为对应的子类。

``Self`` 的前提是方法确实保证返回当前类或其子类的实例。如果实现固定返回某个具体基类，则不应使用 ``Self``。


ClassVar
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

类变量应明确标记为 ``ClassVar``：

.. code-block:: python

   from typing import ClassVar


   class Worker:
       default_timeout: ClassVar[float] = 5.0

       def __init__(self, timeout: float) -> None:
           self.timeout = timeout

``default_timeout`` 属于类，``timeout`` 属于实例。两者在运行时都可以通过属性访问，但在类型语义上并不是同一类状态。


6. 固定结构的数据不要使用裸字典
--------------------------------------------------

字典适合表示映射关系：

.. code-block:: python

   counts: dict[str, int]

这里的键由运行时数据决定，``dict`` 就是正确的抽象。

另一类数据虽然也可以写成字典，但字段实际上是固定的：

.. code-block:: python

   user = {
       "name": "Alice",
       "age": 24,
       "active": True,
   }

如果这种对象需要跨越函数或模块边界，继续使用未声明结构的 ``dict`` 会丢失大量静态信息。字段名拼错、字段缺失或值类型错误，只能更晚发现。

这种数据应当使用具有明确字段定义的结构。


dataclass
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

普通结构化数据优先考虑 ``dataclass``：

.. code-block:: python

   from dataclasses import dataclass


   @dataclass(slots=True)
   class User:
       name: str
       age: int
       active: bool = True

它保留普通 Python 对象的使用方式，同时明确字段和类型。对象形状固定时，可以使用 ``slots=True`` 避免为每个实例保留独立的 ``__dict__``。

如果对象具有少量与数据紧密相关的行为，也可以直接定义在数据类中，没有必要为了“纯数据”再额外建立一层对象。


TypedDict
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

如果数据在运行时本来就应该保持字典形态，可以使用 ``TypedDict``：

.. code-block:: python

   from typing import TypedDict


   class UserData(TypedDict):
       name: str
       age: int
       active: bool

   user: UserData = {
       "name": "Alice",
       "age": 24,
       "active": True,
   }

``TypedDict`` 在运行时仍然只是普通 ``dict``。它增加的是编码阶段的字段约束，而不是运行时校验。

这正符合本文对类型系统的使用方式：让结构在静态阶段清晰，而不强迫运行时对象承担额外机制。


NamedTuple
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

只有数据本身具有 tuple 语义时，才使用 ``NamedTuple``。

例如坐标、固定位置的轻量记录等：

.. code-block:: python

   from typing import NamedTuple


   class Point(NamedTuple):
       x: float
       y: float

如果代码主要通过字段名称访问数据，并且不存在 tuple 语义，通常 ``dataclass`` 更自然。


运行时数据模型
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Pydantic 等模型解决的是另一类问题：运行时解析与验证。

例如 HTTP 请求、配置文件、消息队列内容等数据来自程序控制范围之外，静态检查无法证明这些值符合本地类型声明。这时才需要运行时验证。

因此，可以大致按照下面的方式选择：

.. code-block:: text

   普通结构化内部数据          dataclass
   需要保留 dict 形态          TypedDict
   本身具有 tuple 语义         NamedTuple
   外部输入需要解析或验证      Pydantic 等运行时模型
   真正的动态键值映射          dict[K, V]

选择的依据是数据语义，而不是哪种工具提供的功能最多。


7. 静态类型不是运行时类型系统
--------------------------------------------------

Python 的类型注解首先服务于静态分析。

MyPy、Pylance、IDE 和代码审查可以利用这些信息检查代码，但普通类型注解不会自动阻止下面的调用：

.. code-block:: python

   def add(x: int, y: int) -> int:
       return x + y

   add("a", "b")

类型检查器会报告错误，但 Python 解释器不会因为注解本身拒绝执行这个函数。

这不是类型系统的缺陷，而是 Python 的运行模型。

因此，没有必要为了“落实类型”而在每个函数入口重复进行运行时检查：

.. code-block:: python

   def add(x: int, y: int) -> int:
       if not isinstance(x, int):
           raise TypeError(...)
       if not isinstance(y, int):
           raise TypeError(...)

       return x + y

对于项目内部已经受静态检查约束的代码，这类检查通常只是重复工作。

运行时验证应该放在真正需要它的边界上：

.. code-block:: text

   网络请求
   配置文件
   数据库中的非可信数据
   用户输入
   第三方消息
   动态插件边界

数据通过边界进入受控代码后，内部逻辑继续依靠明确的类型声明和静态检查。

这种划分保留了 Python 的动态运行时，也使类型错误尽可能提前到编码阶段暴露。


结语
--------------------------------------------------

静态类型的价值不在于为 Python 增加一套运行时限制，而在于把原本只存在于开发者脑中的约定写进代码。

类型参数表达类型之间的关系，``ParamSpec`` 保留调用签名，``Protocol`` 描述行为，``Self`` 和 ``ClassVar`` 表达已有的语言语义，``dataclass`` 与 ``TypedDict`` 则让数据结构保持明确。

类型声明应当尽可能准确，但不应该为了追求形式上的完整而破坏 Python 原有的简单性。

能够在静态阶段说明清楚的事情，就在静态阶段说明清楚；确实属于运行时的问题，再交给运行时处理。
