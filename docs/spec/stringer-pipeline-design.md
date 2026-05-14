# Stringer Pipeline Design

日期：2026-05-14

## 目标

Stringer 已经具备导入、导出和基础翻译写回能力。当前缺口在于翻译前后的知识层：术语提示、翻译记录复用、规则化校验、预替换扩展点，以及面向 Agent 的可查询上下文。

本设计新增 `stringer-pipeline`，把这些能力做成独立管线层。第一阶段以 Agent 翻译质量为目标，但 API 按扩展层设计，避免把术语、记忆、规则和 `stringer-workspace` 的文件写回逻辑耦合在一起。

第一阶段范围：

- 覆盖 plugin localization、Bethesda STRINGS 相关条目、Scaleform translation table。
- PEX 暂不接入知识增强和翻译记忆自动复用管线。
- PEX 先在 `stringer-pex` 内提升字符串过滤、锁定、警告和上下文识别。
- 术语默认只作为 hint 和 validation source，不自动替换。
- 显式替换规则预留格式和执行阶段，但第一版不默认启用改写。
- 高置信翻译记忆可以自动填充 package row 的 `translation`；模糊匹配只作为候选提示。

## 非目标

- 不设计 GUI 工作台。
- 不实现外部进程或脚本插件协议。
- 不把 pipeline 直接绑定到某一种文件写回实现。
- 不让 hints 或 diagnostics 影响 `import` 写回语义。
- 不因新增 hints、diagnostics 或 pipeline 元信息升级 translation package schema version。当前软件尚未发布，schema version 只在真正需要破坏兼容边界时变更。

## Crate 边界

依赖方向：

```text
stringer-core
  ↑
stringer-pipeline
  ↑
stringer-workspace
  ↑
stringer-cli
```

`stringer-pipeline` 负责：

- 管线阶段和 processor trait。
- `PipelineEntry`、annotation、diagnostic、report 等统一模型。
- 知识资产加载接口、合并规则和查询接口。
- 内建 processor：术语、翻译记忆、基础校验、预留替换规则。

`stringer-workspace` 负责：

- 把 translation package 转成 pipeline 输入。
- 发现和加载知识目录。
- 提供 public workflow API。
- 读写 package hints、diagnostics 和 translation。

`stringer-cli` 只做薄入口：

- 解析命令行参数。
- 调用 `stringer-workspace` 的 public API。
- 根据需要输出人读文本或结构化 JSON。

## Workspace API

`stringer-workspace` 后续公开以下 API：

```text
annotate_translations(options)
lookup_knowledge(options)
validate_translations(options)
build_knowledge_index(options)
load_knowledge_layers(settings, overrides)
```

现有 API 保持职责：

```text
export_translations(options)
import_translations(options)
```

`import_translations` 只读取 `id` 和 `translation`。它应忽略 hints、diagnostics 和 pipeline metadata，避免提示数据影响写回链路。

`validate_translations` 会重新读取当前知识库并重算 diagnostics，不信任 package 内旧 diagnostics。

## Pipeline 模型

核心类型：

```text
PipelineEntry
PipelineContext
PipelineStage
PipelineProcessor
PipelineAnnotation
PipelineDiagnostic
PipelineReport
```

`PipelineEntry` 是处理器看到的统一条目：

```text
id
kind: plugin | strings | scaleform | pex
source_text
translated_text
source_locale
target_locale
asset_path
context
annotations
diagnostics
```

常见 context：

```text
record_type
subrecord
form_id
strings_kind
string_id
scaleform_key
field_source
storage
```

第一阶段知识 processor 只处理 `plugin`、`strings`、`scaleform`。`pex` 条目可以存在于统一模型中，但相关 processor 默认跳过。

## Pipeline 阶段

管线阶段先完整定义，分批实现：

```text
collect
annotate
pre_translate
memory_apply
post_translate
validate
finalize
```

阶段含义：

- `collect`：把 workspace 或 package row 转为 `PipelineEntry`。
- `annotate`：添加 hints，不改译文。
- `pre_translate`：预留给显式替换、术语保护、token 化等处理。
- `memory_apply`：高置信翻译记忆自动填充。
- `post_translate`：预留给后处理，例如恢复 token。
- `validate`：导入前或提交前校验译文风险。
- `finalize`：汇总 report，供 CLI、Agent 或 GUI 消费。

第一版实现：

- `annotate`：术语命中、翻译记忆候选、知识覆盖提示。
- `memory_apply`：精确 source 和 normalized source 的高置信自动填充。
- `validate`：术语一致性、禁用译法、占位符、Scaleform 换行、空译文策略、记忆冲突。

## Processor 权限

处理器按权限分级：

```text
Annotator: 只能添加 annotations
Suggester: 可以添加候选，但不能写 translated_text
Mutator: 可以写 translated_text，但必须记录 confidence 和来源
```

内建 processor：

- `TerminologyProcessor`：术语命中、推荐译法、禁用译法、术语一致性诊断。
- `TranslationMemoryProcessor`：记忆候选、高置信自动填充、冲突诊断。
- `BasicValidationProcessor`：占位符、Scaleform 换行、空译文策略等基础检查。
- `ReplacementRuleProcessor`：解析显式替换规则和执行点，第一版不默认启用改写。

自动填充只允许高置信结果：

- source 完全一致。
- normalized source 完全一致。
- 上下文加权后置信度达到阈值。

模糊匹配只添加 annotation，不写 `translated_text`。

## Translation Package 扩展

当前 package schema version 不升级。新增字段属于未发布格式的同一版设计演进。

JSONL row 可新增可选字段：

```json
{
  "id": "plugin:MyMod.esp:WEAP:00001234:FULL:1",
  "source": "Iron Sword",
  "translation": "铁剑",
  "context": {
    "record_type": "WEAP",
    "subrecord": "FULL",
    "form_id": "0x00001234"
  },
  "hints": [
    {
      "kind": "term",
      "id": "skyrim.weapon.iron_sword",
      "confidence": 1.0,
      "layer": "project",
      "match": "source",
      "payload": {
        "source": "Iron Sword",
        "target": "铁剑"
      }
    }
  ],
  "diagnostics": [
    {
      "severity": "warning",
      "code": "term.missing",
      "message": "Expected term `Dragonborn` to use `龙裔`."
    }
  ]
}
```

规则：

- `hints` 和 `diagnostics` 均为可选字段。
- `annotate_translations` 可重复运行。
- Pipeline-owned hints 可替换旧值。
- Agent 写入的 `translation` 不应被 annotate 覆盖，除非调用方显式允许 memory auto-fill。
- `validate_translations` 重新计算 diagnostics。
- `import_translations` 忽略 hints、diagnostics 和 pipeline metadata。

## 知识存储

知识资产采用“文件目录为权威，数据库为派生索引”的设计。

项目目录示例：

```text
stringer.toml
knowledge/
  terms/
    base.toml
    skyrim.toml
  rules/
    replacements.toml
  memory/
    project.jsonl
.stringer/
  indexes/
    knowledge.sqlite
```

权威数据：

- `knowledge/terms/*.toml`：术语、别名、推荐译法、禁用译法、范围、备注。
- `knowledge/rules/*.toml`：显式替换规则和启用策略。
- `knowledge/memory/*.jsonl`：翻译记忆，一行一条，适合 Agent 追加和版本控制。

派生数据：

- `.stringer/indexes/knowledge.sqlite`：可删除、可重建。
- 索引文件 fingerprint、归一化 source、context key、term lookup、memory lookup。

如果索引缺失或过期，workspace 可以回退到文件加载和内存索引，并在 report 中给出 warning。

## 知识层合并

加载顺序：

```text
built-in defaults
< user global
< game/language library
< project knowledge
< command-line override
```

后层覆盖前层。跨层覆盖是合法行为，但应输出 diagnostic。示例：

```text
project term `Dragonborn` overrides user-global term `Dragonborn`
```

同一层同一文件内重复 id 是错误。跨层同 id 覆盖是 warning。

## 术语 TOML

示例：

```toml
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "铁剑"
aliases = ["iron sword"]
case_sensitive = false
status = "preferred"
scope = { game = "SkyrimSe", target_locale = "zh-Hans" }
tags = ["weapon", "item-name"]
note = "Used for base-game weapon names."
```

第一版字段：

```text
id
source
target
aliases
case_sensitive
status: preferred | allowed | forbidden
scope: game | source_locale | target_locale | kind | record_type | asset_path
tags
note
```

`preferred` 和 `allowed` 用于提示与校验。`forbidden` 用于标记不应使用的译法，不触发自动替换。

## 显式替换规则 TOML

示例：

```toml
[[rules]]
id = "protect.player_name"
stage = "pre_translate"
pattern = "{PLAYER_NAME}"
replacement = "__STRINGER_TOKEN_PLAYER_NAME__"
mode = "literal"
enabled = false
scope = { kind = ["plugin", "scaleform"] }
note = "Reserved explicit replacement rule."
```

第一版字段：

```text
id
stage: pre_translate | post_translate
mode: literal | regex
pattern
replacement
enabled
scope
note
```

第一版只解析和查询规则。除非调用方显式启用，否则不执行替换。

## 翻译记忆 JSONL

示例：

```json
{"id":"tm:2026-05-14:001","source":"Iron Sword","target":"铁剑","source_locale":"en","target_locale":"zh-Hans","context":{"kind":"plugin","record_type":"WEAP","subrecord":"FULL"},"origin":{"package":"MyMod","entry_id":"plugin:MyMod.esp:WEAP:00001234:FULL:1"},"quality":"confirmed","created_at":"2026-05-14T00:00:00Z"}
```

第一版字段：

```text
id
source
target
source_locale
target_locale
context
origin
quality: confirmed | imported | machine | rejected
created_at
updated_at
```

复用策略：

- `confirmed` 和 `imported` 可作为高置信候选。
- `machine` 默认只作为低置信提示。
- `rejected` 作为负例，用于诊断或降权。

高置信条件：

- source 完全一致，locale 匹配。
- normalized source 完全一致，locale 匹配。
- context 越接近，confidence 越高。

模糊匹配条件：

- 相似 source、大小写差异、标点差异、词序差异等。
- 只输出 candidate annotation。
- 不自动写入 `translated_text`。

## SQLite 索引

建议表结构：

```text
knowledge_files(path, layer, fingerprint, indexed_at)
terms(id, layer, source_norm, target, scope_json, status)
term_aliases(term_id, alias_norm)
memory(id, layer, source_norm, target, context_hash, quality)
memory_context(memory_id, key, value)
```

索引库不是权威数据源。任何时候文件内容与索引冲突，文件胜出。

`stringer knowledge index rebuild` 负责重建索引。普通 lookup 和 annotate 可以在索引不可用时回退到内存索引。

## CLI 工作流

批量 Agent 翻译：

```text
stringer export --root ... --out translations
stringer knowledge annotate --translations translations
Agent edits translation in JSONL rows
stringer knowledge validate --translations translations
stringer import --root ... --translations translations --override-root ...
```

单条查阅：

```text
stringer knowledge lookup --text "Iron Sword" --kind plugin --record-type WEAP --subrecord FULL --json
```

索引维护：

```text
stringer knowledge index rebuild --root ...
```

CLI 命令内部只调用 workspace API，不实现业务逻辑。

## 校验策略

第一版 diagnostics：

```text
term.preferred_missing
term.forbidden_used
placeholder.mismatch
scaleform.newline
translation.empty
memory.conflict
knowledge.override
knowledge.index_stale
processor.skipped
```

严重级别：

```text
error
warning
info
```

`validate` 只报告，不默认阻止 import。后续可在 import 增加显式参数：

```text
--validate
--fail-on warning|error
```

## PEX 独立路线

PEX 第一阶段不进入知识增强和翻译记忆自动复用。原因是 PEX 中字符串既可能是 UI 文本，也可能是函数名、变量名、属性名、内部符号、路径或脚本协议片段。

`stringer-pex` 先补：

- `locked`：明确不可翻译的符号位置、函数名、变量名、property/member 名。
- `warning`：疑似内部标识符、CamelCase、短符号、脚本路径等。
- `call_context`：函数调用目标、member、opcode。
- `concat_context`：已有拼接 group 信息继续保留。

本 spec 不要求立即修改 `StringEntry`。PEX 子设计会先在 `stringer-pex` 内表达以下状态，再评估是否上移到 `stringer-core`：

```text
editable
locked
warning
```

这属于 PEX 子设计，不阻塞 `stringer-pipeline` 第一版。

## 测试计划

`stringer-pipeline`：

- term lookup respects aliases, case sensitivity, scope, and layer order.
- forbidden terms produce diagnostics but do not replace text.
- memory exact and normalized matches can auto-fill with confidence.
- fuzzy memory matches only annotate and never auto-fill.
- replacement rules parse but do not execute by default.
- diagnostics include source layer and rule id.

`stringer-workspace`：

- annotate writes hints without changing package schema version.
- annotate does not overwrite existing Agent translations unless explicitly allowed.
- validate recomputes diagnostics from current knowledge files.
- import ignores hints and diagnostics.
- layer merge order is deterministic.
- stale or missing SQLite index falls back to file-backed lookup.

`stringer-cli`：

- knowledge lookup calls workspace API and can emit JSON.
- knowledge annotate and validate operate on translation packages.
- index rebuild creates or refreshes the derived index.

`stringer-pex`：

- skipped symbol positions are locked or excluded from normal editable flow.
- suspicious identifiers are warnings.
- concat and call context survive extraction and export.

## 默认决策

- Memory auto-fill 的第一版阈值为 `0.95`。完全 source 命中可给 `1.0`；normalized source 命中默认 `0.98`；上下文冲突会降到阈值以下，只保留候选 annotation。
- Package `hints` 至少包含 `kind`、`id`、`layer`、`confidence`、`match` 和 processor-specific payload；内部 processor 字段不序列化到 package。
- Package row 内的 `diagnostics` 至少包含 `severity`、`code`、`message`、`layer` 和可选 `rule_id`；行内不重复输出 `entry_id`。
- `StringEntry` status 不作为 `stringer-pipeline` 第一版前置条件。PEX 状态模型在 PEX 子设计中完成，再决定是否上移到 `stringer-core`。
- CLI 对 memory JSONL 的默认写入策略是 append-only。去重、压缩和归档作为后续维护命令处理。
