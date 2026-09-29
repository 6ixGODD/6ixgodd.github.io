Python: 工程约定
==================================================

:date: 2026-09-29
:slug: python-engineering-conventions
:tags: python, engineering, zh
:lang: zh
:draft: false

本文记录笔者在 Python 项目中的工程约定，供日常开发与 Agent 编码时参考。类型注解另文讨论，此处关注代码组织、依赖关系、开发工具与发布流程。

约定以需要持续维护的项目为对象。示例使用同一组文本读取组件说明接口、组装、调试与测试，不要求所有项目采用相同分层。一次性脚本和只有几行逻辑的工具，没有必要照搬完整结构。


1. 代码风格
--------------------------------------------------

整体参考 Google Python Style Guide，基本原则遵循 Zen of Python。项目中的排版由统一工具处理，不逐项复制另一套 formatter 规则。[google]_ [zen]_

导入优先使用完整包路径，并尽量保留模块限定名。代码中出现一个名称时，应能较容易地判断它来自哪个模块。例如，下面两段代码使用相同的组件：

.. code-block:: python

   # 避免：名称来源需要回到文件顶部查找。
   from ..core.text import TextLoader, TextOptions

   loader = TextLoader(source, options=TextOptions(), name="README.txt")

   # 推荐：完整包路径，使用时保留模块名。
   from project.core import text

   loader = text.TextLoader(
       source,
       options=text.TextOptions(),
       name="README.txt",
   )

示例中的 ``source`` 表示外部传入的数据源，后文给出完整定义。这不是禁止直接导入所有类；类型注解等场景可以按可读性选择。避免的是来源模糊、层级不稳定的导入方式。[google]_

笔者不默认维护 ``__all__``，也不在各级 ``__init__.py`` 中批量重新导出符号。这是本文的项目约定，不是 Google 风格指南的逐字要求。稳定的公共入口可以有选择地导出，内部模块则不必为了缩短几次导入而增加一份名称清单。

命名应准确、简短。优先用包层级表示归属，不靠下划线重复堆叠上下文：

.. code-block:: python

   # 避免重复表达项目、子系统和组件类别。
   from project_internal_text_reader_components import TextReaderComponent

   # 命名空间已经表达了归属，类名只描述职责。
   from project.core import text

   loader: text.TextLoader

能够用一个准确单词表达，就不使用两个；但不为缩短名称引入难以理解的缩写。


2. 源码布局与包边界
--------------------------------------------------

可安装源码放在 ``src/``，测试放在 ``tests/``。普通单包项目可以采用以下布局：[src-layout]_

.. code-block:: text

   project/
   ├── pyproject.toml
   ├── uv.lock
   ├── .ruff.toml
   ├── mypy.ini
   ├── .pre-commit-config.yaml
   ├── src/
   │   └── project/
   │       ├── __init__.py
   │       ├── __main__.py
   │       └── cli.py
   ├── tests/
   └── tools/

``src`` 布局将可安装代码与仓库配置、测试和辅助脚本分开。开发时应正常安装项目，不通过修改 ``sys.path`` 补救导入错误。否则，在仓库中可以运行，并不能说明安装后的包仍然可用。[src-layout]_

多语言仓库在顶层设置 ``python/``，Python 工程放在其中；各发行包内部仍使用 ``src/``。例如单包为 ``python/src/``，多包为 ``python/packages/core/src/``。语言目录与源码目录不是替代关系。


多个发行包共享命名空间
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

SDK、服务端和命令行程序可能具有不同的安装、复用或发布需求。需要划分依赖边界时，可以拆成多个发行包；仅仅有多个启动入口，并不必然需要拆包。

对于属于同一项目语义的多个包，优先考虑 PEP 420 namespace package。例如：

.. code-block:: text

   packages/
   ├── core/
   │   ├── pyproject.toml
   │   └── src/project/core/
   │       ├── __init__.py
   │       ├── text.py
   │       └── files.py
   ├── sdk/
   │   ├── pyproject.toml
   │   └── src/project/sdk/
   │       └── __init__.py
   └── cli/
       ├── pyproject.toml
       └── src/project/cli/
           ├── __init__.py
           ├── __main__.py
           └── main.py

各包的 ``src/project/`` 下均不放置 ``__init__.py``，使 ``project`` 成为共享命名空间。``project.core``、``project.sdk`` 和 ``project.cli`` 则可以分别属于不同发行包。这里不能为了“补齐包结构”而为命名空间添加 ``__init__.py``。[namespace]_

发行名称可以是 ``project-sdk``，导入路径则是 ``project.sdk``。前者用于依赖声明和安装，后者用于 Python 导入。

例如，SDK 和 CLI 都依赖 core，依赖方向应保持明确：

.. code-block:: text

   project.sdk ──→ project.core
   project.cli ──→ project.core


.. code-block:: python

   # SDK 使用核心能力。
   from project.core import text

   # 不应在 core 中反向导入 CLI 的启动逻辑。
   # from project.cli import main

若核心代码需要某种回调或外部行为，应由调用者传入，而不是反向导入调用者所在的包。命名空间只能组织导入名称，不能替代依赖声明或强制执行这些边界。


3. 使用 uv 管理项目
--------------------------------------------------

项目默认使用 uv 管理环境、依赖与锁文件。正式项目按可安装包组织；脚本工程不强制启用打包。以下是不同场景的初始化示例，不是依次执行的步骤：[uv-init]_

.. code-block:: bash

   uv init --package project
   uv init --lib project-lib
   uv init --no-package scripts

多包项目使用 uv workspace，各成员维护自身的 ``pyproject.toml``。根目录声明成员范围：

.. code-block:: toml

   [tool.uv.workspace]
   members = ["packages/*"]

SDK 依赖 core 时，既声明发行依赖，也指定开发时从 workspace 获取它。下面是 ``packages/sdk/pyproject.toml`` 的完整最小示例：[uv-workspace]_ [uv-backend]_

.. code-block:: toml

   [project]
   name = "project-sdk"
   version = "0.1.0"
   requires-python = ">=3.13"
   dependencies = ["project-core>=0.1.0"]

   [tool.uv.sources]
   project-core = { workspace = true }

   [build-system]
   requires = ["uv_build>=0.12.20,<0.13"]
   build-backend = "uv_build"

   [tool.uv.build-backend]
   module-name = "project.sdk"

``project-core`` 是依赖的发行名称，``workspace = true`` 指定本地来源，``module-name`` 则告诉构建后端实际导入路径是 ``project.sdk``。三者处理的是不同问题。

共享 ``uv.lock`` 不等于每次都要安装全部成员。需要使用 CLI 时，可以选择对应成员：[uv-workspace]_

.. code-block:: bash

   uv lock
   uv sync --package project-cli
   uv run --package project-cli python -m project.cli README.rst

锁文件纳入版本控制。CI 使用 ``--locked``，要求锁文件与声明一致，而不是检查过程中自动产生新的依赖结果。[uv-sync]_


依赖组合可解，不要求声明完全相同
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

不同包不需要复制同一套依赖区间。假设 SDK 要求 ``lib>=1,<3``，CLI 要求 ``lib>=2,<4``，只要存在满足共同约束的可用版本，例如 ``2.x``，就可以交给 uv 选择并锁定。

如果两个成员本来就不会一起安装，且依赖存在冲突，可以在支持该功能的 uv 版本中显式声明互斥关系。以下片段假设 ``project-legacy`` 与 ``project-modern`` 已是 workspace 成员：[uv-resolution]_

.. code-block:: toml

   [tool.uv]
   conflicts = [
       [
           { package = "project-legacy" },
           { package = "project-modern" },
       ],
   ]

uv 可以分别解析允许的组合，但不能因此把这两个互斥成员安装进同一个环境。未声明互斥时，解析器不会自行推断“开发者大概不会同时使用它们”。实际需要共同安装的组合，仍必须满足依赖约束。[uv-resolution]_

因此，工程上的要求是准确声明依赖、环境条件和必要的互斥关系，以允许的组合可解为准；不要求所有包的所有依赖采用相同版本。

workspace 也不自动验证依赖声明是否完整。例如 SDK 漏写一个依赖，但共享环境中恰好由 CLI 安装了它，SDK 的测试仍可能通过。需要独立分发的包，应额外在只安装自身声明依赖的环境中验证。[uv-workspace]_


4. 程序入口
--------------------------------------------------

正式项目中的可执行包提供 ``__main__.py``，支持 ``python -m``。入口只负责转交控制，不在其中堆积实现，也不依赖从仓库根目录执行某个脚本。[main]_

对于前面的 CLI 包：

.. code-block:: python

   from project.cli import main

   if __name__ == "__main__":
       raise SystemExit(main.main())

``main.py`` 负责参数、对象组装和退出码。以下命令读取指定文件并输出文本；所使用的核心组件在下一节定义：

.. code-block:: python

   import logging
   import sys

   from project.core import files, text

   logger = logging.getLogger(__name__)


   def main() -> int:
       if len(sys.argv) != 2:
           print("usage: project-text FILE", file=sys.stderr)
           return 2

       loader = text.TextLoader(
           files.FileSource(),
           options=text.TextOptions(),
           name=sys.argv[1],
       )
       logger.debug("Initialized %r", loader)

       try:
           result = loader.load()
       except (OSError, UnicodeError) as error:
           print(error, file=sys.stderr)
           return 1

       sys.stdout.write(result)
       return 0

安装后的命令与 ``python -m`` 共用同一函数：

.. code-block:: toml

   [project.scripts]
   project-text = "project.cli.main:main"


.. code-block:: bash

   uv run --package project-cli python -m project.cli README.rst
   uv run --package project-cli project-text README.rst

核心组件负责读取和解码，入口负责将失败转换成命令行错误与退出码。库代码不应直接调用 ``sys.exit``，否则它的使用者无法决定如何处理失败。[main]_


5. 依赖关系与对象设计
--------------------------------------------------

在 SOLID 原则中，笔者首先关注依赖倒置：高层逻辑依赖所需的抽象，而不是直接绑定某个底层实现。构造器注入是落实依赖关系的一种方式，但仅仅增加一个构造参数，并不等于已经建立了合理抽象。[dip]_ [injection]_

以文本读取为例。如果组件在内部创建文件数据源：

.. code-block:: python

   from project.core import files


   class TextLoader:
       def __init__(self, name: str) -> None:
           self._source = files.FileSource()
           self._name = name

       def load(self) -> str:
           return self._source.read(self._name).decode("utf-8")

解码逻辑便与文件读取绑定。换成内存或远程数据源时，需要修改该类；单元测试也必须使用文件，或者替换模块内部引用。

当输入来源确实需要替换时，应先明确组件真正需要的行为：根据名称取得字节。``ByteSource`` 描述这个接口，``TextLoader`` 接收其实现，而不导入具体数据源：

.. code-block:: python

   from dataclasses import dataclass
   from typing import Literal, Protocol


   class ByteSource(Protocol):
       def read(self, name: str, /) -> bytes: ...


   @dataclass(frozen=True, slots=True)
   class TextOptions:
       encoding: str = "utf-8"
       errors: Literal["strict", "replace", "ignore"] = "strict"


   class TextLoader:
       def __init__(
           self,
           source: ByteSource,
           *,
           options: TextOptions,
           name: str,
       ) -> None:
           self._source = source
           self._options = options
           self._name = name

       def load(self) -> str:
           data = self._source.read(self._name)
           return data.decode(
               self._options.encoding,
               errors=self._options.errors,
           )

这里的构造器对应三类参数：``source`` 是依赖组件，``options`` 是一组读取配置，``name`` 是本次实例处理的资源名称。三者不必包装进同一个总配置对象。

``encoding`` 与 ``errors`` 共同决定解码行为，放在 ``TextOptions`` 中有明确含义；若某个组件根本没有成组配置，不必为了统一构造器形式创建配置类。实例差异也不必都归入配置：这里两个 loader 可以共享选项，但读取不同名称。

文件实现只负责获得字节，不需要继承 ``TextLoader``，也不需要显式继承 Protocol：

.. code-block:: python

   import pathlib


   class FileSource:
       def read(self, name: str, /) -> bytes:
           return pathlib.Path(name).read_bytes()

运行时由外部组装：

.. code-block:: python

   from project.core import files, text

   source = files.FileSource()
   options = text.TextOptions(encoding="utf-8", errors="strict")

   first = text.TextLoader(source, options=options, name="README.txt")
   second = text.TextLoader(source, options=options, name="LICENSE.txt")

组装代码需要了解具体实现，使用它的 ``TextLoader`` 不需要。输入来源变更时，调整实现和组装位置即可；不必让核心逻辑到全局容器中自行查找依赖。

构造器只保存完成工作所需的信息，实际读取发生在 ``load()`` 中。这样创建实例、查看实例和执行 I/O 的时机能够分开控制。普通局部对象仍可在组件内部创建，不要求把每个辅助对象都变成注入参数。

这段代码用于说明可替换边界，并不意味着一次 ``Path.read_text()`` 必须写成三个类。是否引入接口，取决于实际的替换、复用和测试需求。


用 __repr__ 表达必要的实例信息
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

建议重要组件实现 ``__repr__``，使调试器与日志能够显示有用的实例摘要。Python 将这一表示主要用于调试；不要求每个对象的表示都能直接复制为重建表达式。[repr]_

将下面的方法加入前面的 ``TextLoader``：

.. code-block:: python

   def __repr__(self) -> str:
       return (
           f"{type(self).__name__}("
           f"name={self._name!r}, "
           f"options={self._options!r}, "
           f"source={type(self._source).__name__})"
       )

表示结果类似：

.. code-block:: text

   TextLoader(name='README.txt', options=TextOptions(encoding='utf-8', errors='strict'), source=FileSource)

这里能看到实例处理的资源、解码方式和数据源类型，但没有读取文件，也没有输出数据内容。

不应为了生成表示而执行读取，例如：

.. code-block:: python

   # 不推荐：在调试器中查看对象就可能触发 I/O，甚至抛出异常。
   def __repr__(self) -> str:
       data = self._source.read(self._name)
       return f"TextLoader(size={len(data)})"

``__repr__`` 应无副作用、成本可控；依赖只显示类型或稳定标识，不递归展开整个对象图。已有自动生成的表示能够满足需要时，不必重复实现。敏感字段和大型数据也不应直接输出。

初始化等关键节点可复用该表示：

.. code-block:: python

   logger.debug("Initialized %r", loader)

使用日志参数形式，不提前构造 ``f"{loader!r}"``。日志库可以将消息格式化延迟到需要输出时，但传入参数之前执行的函数调用仍会立即发生。[logging]_

因此，即使采用参数形式，也不应写成 ``logger.debug("Loaded %s", loader.load())``，把工作执行隐藏在日志表达式里。日志记录已经发生的事件，不负责驱动组件行为。


6. 静态检查与工具配置
--------------------------------------------------

类型检查使用 mypy 或 Pyright，具体规则沿用类型系统一文。其余常见 lint、导入排序与格式化默认由 Ruff 处理，不再同时维护职责重叠的 isort、YAPF 等配置。

``ruff format`` 不负责导入排序；排序由 lint 的 ``I`` 规则完成。因此先修复，再格式化：[ruff-format]_

.. code-block:: bash

   uv run ruff check --fix .
   uv run ruff format .

以下 ``.ruff.toml`` 对应支持 Python 3.13 及以上的多包示例。实际项目按最低支持版本调整 ``target-version``：[ruff-config]_

.. code-block:: toml

   target-version = "py313"
   line-length = 80
   src = ["packages/*/src"]

   [lint]
   extend-select = ["I"]

这里只是在默认 lint 规则外启用导入排序，不代表完整实现了 Google 风格指南。Ruff formatter 主要遵循与 Black 兼容的排版方式；项目采用统一 formatter 后，不应再叠加相互冲突的排版规则。[ruff-format]_

如果团队需要 Ruff 未覆盖的检查，再考虑 Pylint。笔者通常先关闭与项目不匹配、反馈价值较低的规则，不要求满足所有默认提示；但规则取舍应有理由，不能只为获得“全绿”而整体关闭有意义的检查。


工具配置独立维护
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

尽可能将工具配置从 ``pyproject.toml`` 拆出。同一项配置只有一处，不因工具支持多种格式就在几个文件中重复声明。例如：

.. code-block:: text

   pyproject.toml          项目元数据、依赖、构建、入口与 workspace
   .ruff.toml             lint 与格式化
   mypy.ini               类型检查
   pytest.ini             测试发现与 pytest 行为
   .coveragerc            覆盖范围与门槛
   .pre-commit-config.yaml 提交前检查

独立配置文件使用工具规定的结构，不能直接搬运带前缀的 TOML 表名。例如 ``pyproject.toml`` 中的 ``[tool.ruff.lint]``，移至 ``.ruff.toml`` 后应改成 ``[lint]``。[ruff-config]_

多包类型检查还需使检查器正确理解命名空间和源码根。以下 ``mypy.ini`` 只展示布局与严格模式，类型规则按上一篇文章继续补充：[mypy-config]_

.. code-block:: ini

   [mypy]
   python_version = 3.13
   strict = True
   namespace_packages = True
   explicit_package_bases = True
   mypy_path = packages/core/src,packages/sdk/src,packages/cli/src
   files = packages,tests,tools
   show_error_codes = True

这些路径只影响 mypy 如何定位模块，不是运行时修改 ``sys.path`` 的替代方案。运行时仍通过正常安装获得包。


7. Pre-commit 保持最小配置
--------------------------------------------------

建议使用 pre-commit，在提交前处理低成本、反馈明确的问题。不将完整构建和测试流程全部放进本地 hook。[precommit]_

默认保留文件末尾换行、行尾空白、Ruff 修复与格式化，以及 ``pyproject.toml`` 格式化。以下是固定版本示例，不要求项目永远停留在这些版本：[hooks]_ [ruff-hooks]_ [pyproject-fmt]_

.. code-block:: yaml

   repos:
     - repo: https://github.com/pre-commit/pre-commit-hooks
       rev: v6.0.0
       hooks:
         - id: end-of-file-fixer
         - id: trailing-whitespace

     - repo: https://github.com/astral-sh/ruff-pre-commit
       rev: v0.16.9
       hooks:
         - id: ruff-check
           args: [--fix]
         - id: ruff-format

     - repo: https://github.com/tox-dev/pyproject-fmt
       rev: v2.29.4
       hooks:
         - id: pyproject-fmt
           files: '(^|/)pyproject\.toml$'

Ruff 的修复在格式化之前执行，避免修复后留下新的格式差异。本地依赖、hook 与 CI 的 Ruff 版本应一致。[ruff-hooks]_

项目涉及凭证时，可在 ``repos`` 中增加：

.. code-block:: yaml

   - repo: https://github.com/Yelp/detect-secrets
     rev: v1.5.0
     hooks:
       - id: detect-secrets

误报按具体位置处理，不将真实密钥加入忽略清单。检测工具用于降低误提交概率，并不替代凭证的正确存储方式。[secrets]_

把 pre-commit 声明为开发依赖后，安装 hook 并执行一次全量检查：

.. code-block:: bash

   uv run pre-commit install
   uv run pre-commit run --all-files

修复型 hook 修改文件后，需要检查差异并重新暂存，不在 hook 中自动执行 ``git add``。本地 hook 可以被跳过，因此关键质量检查仍须在 CI 中执行。[precommit]_


8. 单元测试与持续集成
--------------------------------------------------

新增行为有对应测试，修复缺陷时保留回归用例。测试至少检查有意义的结果、边界与错误路径，不以“函数执行过”代替断言。

前面的依赖注入在这里有直接用途。测试 ``TextLoader`` 的解码行为不需要创建文件，也不需要 patch 模块内部变量：提供一个符合接口的内存数据源即可。以下内容保存为 ``tests/test_text.py``；参数化测试用于复用同一项行为检查。[pytest]_

.. code-block:: python

   from dataclasses import dataclass

   import pytest

   from project.core import text


   @dataclass
   class MemorySource:
       data: bytes
       requested: str | None = None

       def read(self, name: str, /) -> bytes:
           self.requested = name
           return self.data


   @pytest.mark.parametrize(
       ("raw", "expected"),
       [(b"", ""), (b"hello\n", "hello\n"), ("你好".encode(), "你好")],
   )
   def test_load(raw: bytes, expected: str) -> None:
       source = MemorySource(raw)
       loader = text.TextLoader(
           source,
           options=text.TextOptions(),
           name="sample.txt",
       )

       assert loader.load() == expected
       assert source.requested == "sample.txt"


   def test_invalid_utf8_raises() -> None:
       loader = text.TextLoader(
           MemorySource(b"\xff"),
           options=text.TextOptions(),
           name="broken.txt",
       )

       with pytest.raises(UnicodeDecodeError):
           loader.load()


   def test_invalid_utf8_can_be_replaced() -> None:
       loader = text.TextLoader(
           MemorySource(b"\xff"),
           options=text.TextOptions(errors="replace"),
           name="broken.txt",
       )

       assert loader.load() == "\ufffd"

正常用例覆盖空数据、换行与多字节字符，并检查资源名称是否正确传入依赖。错误用例分别验证严格解码失败和替换策略。预期值明确，不是对实现再执行一遍后拿同样的结果作比较。

仅使用 ``assert loader.load() is not None``，即使覆盖了同一行代码，也无法发现返回空字符串、丢失换行或忽略解码策略等错误。

调试表示无副作用同样可以被验证。仍沿用上面的 ``MemorySource``：

.. code-block:: python

   def test_repr_does_not_read_data() -> None:
       source = MemorySource(b"payload")
       loader = text.TextLoader(
           source,
           options=text.TextOptions(),
           name="sample.txt",
       )

       description = repr(loader)

       assert source.requested is None
       assert "sample.txt" in description
       assert "MemorySource" in description
       assert "payload" not in description

这里的 ``MemorySource`` 只替代输入来源，不替代被测的 ``load()`` 或 ``__repr__``。具体文件适配器应另行使用临时文件测试；CLI 还需要检查输出和退出码。不能因为核心单元测试通过，就省略安装或入口验证。

AI 参与编码时，应将测试纳入同一次修改。可以让 Agent 枚举边界、补充错误路径和回归用例，但预期结果应来自需求或独立推导，不能只将当前实现的输出保存为断言。节省的编码时间不应以减少验证为代价。


覆盖率需要明确范围
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

覆盖率设项目级门槛，同时关注分支覆盖。以下配置只统计示例的 core 包，门槛 ``90`` 是演示值，不是所有项目的统一标准：[coverage]_

.. code-block:: ini

   [run]
   branch = True
   source =
       project.core

   [report]
   fail_under = 90
   show_missing = True

实际项目应列出需要验证的所有包，不能只挑已经容易达到门槛的模块。工具脚本若承担版本同步等关键职责，也应有测试；它们可以另设覆盖范围，而不是因位于 ``tools/`` 就排除验证。

执行时明确使用该配置：

.. code-block:: bash

   uv run --all-packages pytest --cov --cov-config=.coveragerc

``--cov`` 不在命令行另给来源，从而沿用配置中的 ``source``。覆盖率能够显示代码未经过的路径，但不能证明断言足以发现错误。[pytest-cov]_


CI 检查，而非静默修复
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

对于上面不存在互斥成员的示例 workspace，CI 可以执行：

.. code-block:: bash

   uv sync --all-packages --locked
   uv run --locked ruff check .
   uv run --locked ruff format --check .
   uv run --all-packages --locked mypy
   uv run --all-packages --locked pytest --cov --cov-config=.coveragerc
   uv run --locked python tools/sync_version.py --check
   uv build --all-packages

类型检查和测试在这里选择全部示例成员。若 workspace 包含已声明的互斥成员，应按允许的组合分开执行，不能照搬 ``--all-packages``。[uv-resolution]_

CI 不使用 ``ruff check --fix`` 或 ``ruff format`` 隐式修复提交。格式变化应由开发者或 Agent 提交，CI 只判断当前提交是否满足约定。

对外分发的包还应在干净环境安装构建产物。以下以 Unix 类环境和 core 的 wheel 为例：

.. code-block:: bash

   uv venv .venv-wheel
   uv pip install --python .venv-wheel/bin/python dist/project_core-*.whl
   .venv-wheel/bin/python -I -c "from project.core import text; print(text.TextOptions())"

``-I`` 避免从当前目录或 ``PYTHONPATH`` 导入源码；检查对象是安装后的包，而不是开发环境恰好能找到的文件。[python-cli]_


9. 版本与发布
--------------------------------------------------

有发布需求的项目维护 ``VERSION`` 文本文件，作为唯一人工修改的版本来源。``pyproject.toml`` 中的版本和包内 ``__version__`` 可以保留，但由脚本同步，不分别手工维护。[version]_

以多个包统一发布为例：

.. code-block:: text

   VERSION                                      0.1.0
   packages/core/pyproject.toml                  project.version = "0.1.0"
   packages/core/src/project/core/_version.py    __version__ = "0.1.0"
   packages/sdk/pyproject.toml                   project.version = "0.1.0"
   packages/sdk/src/project/sdk/_version.py      __version__ = "0.1.0"
   packages/cli/pyproject.toml                   project.version = "0.1.0"
   packages/cli/src/project/cli/_version.py      __version__ = "0.1.0"

``_version.py`` 是脚本生成的专用文件，不在里面维护其他逻辑。需要保留 ``project.core.__version__`` 这个公开属性时，在该包的 ``__init__.py`` 中显式导出：

.. code-block:: python

   from project.core._version import __version__ as __version__

这是有明确用途的公开导出，不要求顺便重导出其他类和函数。也可以让脚本更新现有 ``__init__.py`` 中的赋值，但应准确定位该声明，而不是覆盖整个文件。


同步目标限定到文件和字段
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

不能把仓库中所有 ``0.1.0`` 替换成新版本。相同字符串也可能是第三方依赖约束、文档中的历史版本或测试输入。

以下 ``tools/sync_version.py`` 明确列出参与统一发布的包，只更新 ``project.version`` 和专用 ``_version.py``。TOML Kit 用于保留 TOML 注释及排版；``packaging`` 用于验证版本格式。这两个工具仅作为开发依赖，不进入运行时依赖。[tomlkit]_ [packaging-version]_

.. code-block:: python

   import argparse
   import pathlib
   import sys
   from collections.abc import MutableMapping

   import tomlkit
   from packaging.version import Version

   ROOT = pathlib.Path(__file__).resolve().parents[1]
   PACKAGES = ("core", "sdk", "cli")


   def plan(root: pathlib.Path) -> dict[pathlib.Path, str]:
       version = (root / "VERSION").read_text(encoding="utf-8").strip()
       if str(Version(version)) != version:
           raise ValueError("VERSION must contain a canonical Python version")

       changes: dict[pathlib.Path, str] = {}
       for name in PACKAGES:
           package = root / "packages" / name
           metadata = package / "pyproject.toml"
           document = tomlkit.parse(metadata.read_text(encoding="utf-8"))
           project = document.get("project")
           if not isinstance(project, MutableMapping) or "version" not in project:
               raise ValueError(f"Missing static project.version: {metadata}")

           project["version"] = version
           changes[metadata] = tomlkit.dumps(document)
           target = package / "src" / "project" / name / "_version.py"
           if not target.parent.is_dir():
               raise ValueError(f"Missing package directory: {target.parent}")
           changes[target] = (
               "# Generated from VERSION; do not edit.\n"
               f'__version__ = "{version}"\n'
           )
       return changes


   def synchronize(root: pathlib.Path, *, check: bool) -> int:
       pending = []
       # Validate every target before writing any file.
       for path, expected in plan(root).items():
           actual = path.read_text(encoding="utf-8") if path.exists() else None
           if actual != expected:
               pending.append((path, expected))

       for path, expected in pending:
           if check:
               print(f"Version mismatch: {path.relative_to(root)}", file=sys.stderr)
           else:
               path.write_text(expected, encoding="utf-8")
               print(f"Updated: {path.relative_to(root)}")
       return 1 if check and pending else 0


   class Arguments(argparse.Namespace):
       check: bool


   def main() -> int:
       parser = argparse.ArgumentParser()
       parser.add_argument("--check", action="store_true")
       args = parser.parse_args(namespace=Arguments())
       return synchronize(ROOT, check=args.check)


   if __name__ == "__main__":
       try:
           raise SystemExit(main())
       except (OSError, ValueError) as error:
           raise SystemExit(str(error)) from error

先计算全部目标内容，再开始写入，能够避免后面的配置错误导致前面文件已经被更新。脚本只在内容发生变化时写文件，重复执行不会产生新差异；但它不是跨文件事务，写入中断时仍需重新执行或检查版本控制差异。

本例要求各包已有静态 ``project.version``。若项目采用构建后端动态版本，就不再把同一字段配置成静态同步目标，避免两种机制同时负责同一个版本声明。

开发时同步，CI 中只检查：

.. code-block:: bash

   # 修改 VERSION 后，更新其他声明。
   uv run --no-sync python tools/sync_version.py
   uv lock

   # CI：发现不一致就失败，不改写被检查的提交。
   uv run --locked python tools/sync_version.py --check

同步修改了项目元数据后，还要更新并提交 ``uv.lock``。``--no-sync`` 用于版本修改过程，避免运行脚本前先由 uv 隐式同步环境；前提是开发环境已经安装了脚本依赖。CI 使用 ``--locked``，不能忽略源码版本与锁文件的差异。[uv-sync]_

也可以把同步模式加入 pre-commit 的 ``repos``：

.. code-block:: yaml

   - repo: local
     hooks:
       - id: sync-version
         name: Synchronize package versions
         entry: uv run --no-sync python tools/sync_version.py
         language: system
         pass_filenames: false
         always_run: true

hook 不自行提交生成结果。开发者检查变更、更新锁文件并重新暂存，CI 再校验一次。版本同步脚本本身也应测试：至少验证版本漂移能够被发现、检查模式不写文件、重复执行结果不变，以及依赖约束不会被误改。

多包独立发布时，按发布单元分别维护 ``VERSION`` 和同步范围，不因共用 workspace 就强制采用同一个版本号。


发布构建后的同一份产物
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

版本文件变更可以作为 CI 的发布触发条件。例如，仅在 ``main`` 上的 ``VERSION`` 变更或人工触发时启动发布工作流：[github-actions]_

.. code-block:: yaml

   on:
     push:
       branches: [main]
       paths: [VERSION]
     workflow_dispatch:

这只是触发片段，检查步骤仍然需要完整执行。不要仅因文件发生变化，就跳过测试或版本校验。

.. code-block:: text

   VERSION 变更
       → 同步结果与锁文件校验
       → 静态检查、测试、构建
       → 安装产物并核对版本
       → 为通过验证的提交创建 tag
       → 发布同一次构建的产物

安装后可检查包内声明与发行元数据是否一致，避免只对比源码中的字符串：[version]_

.. code-block:: python

   from importlib import metadata

   from project import core

   assert core.__version__ == metadata.version("project-core")

发布流程还应将这个版本与 ``VERSION`` 对照。运行时查询元数据是验证手段，不替代本文选择的版本同步方式。

已有 tag 不移动到新提交；已发布版本不覆盖。CI 重跑时，应区分“同一提交的发布重试”和“试图用同一个版本发布不同代码”。独立发布的包还应把包名纳入 tag，例如 ``core-v0.1.0``。

.. note:: GitHub Actions 的 tag 触发

   默认 ``GITHUB_TOKEN`` 推送 tag，不会因此启动另一个监听该 push 的工作流。
   可在同一工作流内继续发布，或显式安排后续工作流，不能假定创建 tag
   会自动衔接第二条流水线。[github-actions]_


结语
--------------------------------------------------

代码边界通过导入和接口体现，依赖通过项目声明与构造器体现，行为通过测试验证，版本通过脚本保持一致。

需要持续维护的，不只是代码，还包括这些约定的执行方式。能够自动检查的部分应落实到工具与 CI；不需要的分层和机制则不必引入。


参考资料
--------------------------------------------------

.. [google] `Google Python Style Guide <https://google.github.io/styleguide/pyguide.html>`_

.. [zen] `PEP 20 — The Zen of Python <https://peps.python.org/pep-0020/>`_

.. [src-layout] `PyPA：src layout vs flat layout <https://packaging.python.org/en/latest/discussions/src-layout-vs-flat-layout/>`_

.. [namespace] `PyPA：Packaging namespace packages <https://packaging.python.org/en/latest/guides/packaging-namespace-packages/>`_

.. [uv-init] `uv：Creating projects <https://docs.astral.sh/uv/concepts/projects/init/>`_

.. [uv-workspace] `uv：Using workspaces <https://docs.astral.sh/uv/concepts/projects/workspaces/>`_

.. [uv-backend] `uv：Build backend <https://docs.astral.sh/uv/concepts/build-backend/>`_

.. [uv-resolution] `uv：Conflicting dependencies <https://docs.astral.sh/uv/concepts/resolution/#conflicting-dependencies>`_

.. [uv-sync] `uv：Locking and syncing <https://docs.astral.sh/uv/concepts/projects/sync/>`_

.. [main] `Python：__main__ <https://docs.python.org/3/library/__main__.html>`_

.. [dip] `Robert C. Martin：A Little Architecture <https://blog.cleancoder.com/uncle-bob/2016/01/04/ALittleArchitecture.html>`_

.. [injection] `Martin Fowler：Dependency Injection <https://martinfowler.com/articles/injection.html>`_

.. [repr] `Python：object.__repr__ <https://docs.python.org/3/reference/datamodel.html#object.__repr__>`_

.. [logging] `Python：Logging HOWTO <https://docs.python.org/3/howto/logging.html>`_

.. [ruff-format] `Ruff：The Ruff Formatter <https://docs.astral.sh/ruff/formatter/>`_

.. [ruff-config] `Ruff：Configuring Ruff <https://docs.astral.sh/ruff/configuration/>`_

.. [mypy-config] `mypy：The configuration file <https://mypy.readthedocs.io/en/stable/config_file.html>`_

.. [precommit] `pre-commit <https://pre-commit.com/>`_

.. [hooks] `pre-commit-hooks <https://github.com/pre-commit/pre-commit-hooks>`_

.. [ruff-hooks] `Ruff：pre-commit integration <https://docs.astral.sh/ruff/integrations/#pre-commit>`_

.. [pyproject-fmt] `pyproject-fmt <https://github.com/tox-dev/pyproject-fmt>`_

.. [secrets] `detect-secrets <https://github.com/Yelp/detect-secrets>`_

.. [pytest] `pytest：Parametrizing tests <https://docs.pytest.org/en/stable/how-to/parametrize.html>`_

.. [coverage] `Coverage.py：Configuration reference <https://coverage.readthedocs.io/en/latest/config.html>`_

.. [pytest-cov] `pytest-cov：Configuration <https://pytest-cov.readthedocs.io/en/latest/config.html>`_

.. [python-cli] `Python：Command line and environment <https://docs.python.org/3/using/cmdline.html>`_

.. [version] `PyPA：Single-sourcing the Project Version <https://packaging.python.org/en/latest/discussions/single-source-version/>`_

.. [tomlkit] `TOML Kit：Quickstart <https://tomlkit.readthedocs.io/en/latest/quickstart/>`_

.. [packaging-version] `Packaging：Version handling <https://packaging.pypa.io/en/stable/version.html>`_

.. [github-actions] `GitHub Actions：Triggering a workflow <https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow>`_
