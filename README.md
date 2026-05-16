# Stringer

Stringer 是一个面向 Bethesda 模组本地化的命令行工具和 Rust workspace。它把可翻译文本从模组目录打开成适合人工或 Agent 编辑的翻译工作区，再把完成后的译文写到覆盖目录；同时提供术语、翻译记忆、规则校验和单条查询能力，帮助 Agent 在没有额外说明的情况下判断怎么翻译、怎么检查、怎么完成工作区。

当前主要覆盖：

- Plugin 本地化文本：`.esp`、`.esm`、`.esl` 以及配套 `.strings`、`.dlstrings`、`.ilstrings`。
- PEX 脚本字符串：`.pex`。
- Scaleform 翻译表：`Interface/Translations/*.txt`。
- 知识层：术语表、翻译记忆、替换规则、派生 SQLite 索引。
- Adapt：把 xTranslator / ESP-ESM Translator 旧翻译资源转换为 Stringer 翻译记忆。

## 构建与检查

项目使用 Rust workspace，最低 Rust 版本见 `Cargo.toml`。

```powershell
cargo build
cargo test
```

提交前按项目规则运行：

```powershell
cargo fmt
cargo clippy --workspace --all-targets
cargo test
cargo xtask line-budget
```

## 基本工作流

推荐 Agent 在翻译工作区目录运行；`--workspace` 可省略，默认就是当前目录。下面示例保留 `--workspace` 以便从仓库根目录直接运行。

1. 从模组目录打开翻译工作区：

```powershell
cargo run -p stringer-cli -- workspace open `
  --source-root path/to/mod-root `
  --workspace path/to/translations `
  --game-release SkyrimSe `
  --asset-language English `
  --source-locale en `
  --target-locale zh-Hans
```

2. 用知识库给翻译工作区写入提示：

```powershell
cargo run -p stringer-cli -- knowledge annotate `
  --workspace path/to/translations
```

3. 统计并认领 Agent 翻译批次：

```powershell
cargo run -p stringer-cli -- workspace batch count `
  --workspace path/to/translations `
  --json

cargo run -p stringer-cli -- workspace inspect diagnostics `
  --workspace path/to/translations `
  --severity warning

cargo run -p stringer-cli -- workspace batch claim `
  --workspace path/to/translations `
  --limit 50

cargo run -p stringer-cli -- workspace inspect batch `
  --workspace path/to/translations `
  --batch-id b1770000000000-1234 `
  --limit 10 `
  --offset 0
```

`claim` 只负责认领，会输出紧凑 JSON，包含 `batch_id`、`claimed_entries` 和认领 scope。用 `workspace inspect batch` 按页读取该 batch 的原文、当前译文、上下文、`hints` 和 `diagnostics`。Agent 翻译后用一次或多次 JSON patch 回写；每次回写后从 `--offset 0` 重新读取剩余 batch，因为已应用的条目会从 batch 中移除：

```json
{"batch_id":"b1770000000000-1234","entries":[{"id":"plugin:Example.esp:WEAP:0x00001234:FULL:0","translation":"铁剑"}]}
```

```powershell
cargo run -p stringer-cli -- workspace batch apply `
  --workspace path/to/translations `
  --input path/to/patch.json
```

4. 校验翻译工作区：

```powershell
cargo run -p stringer-cli -- knowledge validate `
  --workspace path/to/translations
```

5. 完成翻译工作区并写回覆盖目录：

```powershell
cargo run -p stringer-cli -- workspace finalize `
  --workspace path/to/translations `
  --output path/to/translations/output
```

`workspace finalize` 默认读取 `workspace.json` 里保存的 `source_root`，并把结果写到 `<workspace>/output`。`--source-root` 只用于显式覆盖已保存的源目录；`--output` 必须位于源模组目录外。

## 翻译工作区结构

`workspace open` 输出的是一个目录，不是单个文件。Workspace v4 使用 `workspace.json`、`batches/`、`entries/**/*.jsonl`、`knowledge/` 和默认 `output/`。

```text
translations/
  workspace.json
  batches/
  entries/
    plugin/<asset>/<record_type>.jsonl
    pex/<asset>.jsonl
    scaleform/<asset>.jsonl
  knowledge/
    terms/
    memory/
    rules/
    index.sqlite
  output/
```

`workspace.json` 记录 schema、只读 `source_root`、游戏版本、资产语言、locale 和条目文件列表。旧版 `manifest.json` 工作区不会被 Workspace v4 命令读取；当前 `workspace upgrade` 只报告迁移尚未实现，旧工作区需要用 `workspace open` 重建。`entries/**/*.jsonl` 每行一个记录，常见字段如下：

- `id`：稳定条目 ID，完成工作区时用它定位源文本。
- `source`：源文本，不建议改。
- `translation`：译文；缺失时完成工作区会跳过该条。
- `translation_meta`：译文来源，例如 `memory` 或 `agent`。
- `context`：记录类型、子记录、Form ID、Scaleform key、PEX 调用位置等上下文。
- `hints`：`knowledge annotate` 写入的术语和记忆提示。
- `diagnostics`：`knowledge validate` 写入的校验结果。

`workspace finalize` 只读取 `id` 和 `translation`。`hints`、`diagnostics` 和其他扩展字段不会影响写回。

直接编辑 `entries/**/*.jsonl` 仍可作为人工 fallback。Agent 应优先使用 `workspace inspect ...` 做只读查看和审阅，并使用 `workspace batch count/claim/apply/release` 写入翻译，避免破坏 JSONL 格式或频繁逐行调用 CLI。

## 工作区只读查看

`workspace inspect` 提供结构化 JSON 输出，用于让 Agent 查看工作区而不直接读取 `workspace.json`、`entries/**/*.jsonl` 或 `batches/*.json`：

```powershell
cargo run -p stringer-cli -- workspace inspect files `
  --workspace path/to/translations

cargo run -p stringer-cli -- workspace inspect entries `
  --workspace path/to/translations `
  --status diagnostic `
  --limit 50

cargo run -p stringer-cli -- workspace inspect entry `
  --workspace path/to/translations `
  --id "plugin:Example.esp:WEAP:0x00001234:FULL:0"

cargo run -p stringer-cli -- workspace inspect batch `
  --workspace path/to/translations `
  --batch-id b1770000000000-1234 `
  --limit 10 `
  --offset 0

cargo run -p stringer-cli -- workspace inspect diagnostics `
  --workspace path/to/translations `
  --severity warning
```

`entries --status` 支持 `all`、`empty`、`memory`、`translated`、`claimed` 和 `diagnostic`。`diagnostics --severity` 支持 `all`、`error`、`warning` 和 `info`。`inspect batch` 支持 `--limit` 和 `--offset`，返回当前剩余 batch 的 `total`；如果中途 apply，下一次应从 `--offset 0` 读取剩余条目。Inspect 命令只读，不会创建 claim、释放 batch 或写入译文。

## 配置

CLI 支持从默认用户配置文件和 workspace `stringer.toml` 读取基础设置，也支持命令行覆盖。覆盖顺序为：用户配置 < workspace 配置 < 命令行参数。`workspace open` 只读取 `<workspace>/stringer.toml`，不会读取 source root 下的配置。为了让 Agent 自洽，推荐显式传入这些参数，或把固定设置写入 workspace `stringer.toml`：

- `--game-release`：`SkyrimLe` 或 `SkyrimSe`。
- `--asset-language`：Bethesda 资产语言，例如 `English`、`Chinese`、`ChineseSimplified`。
- `--source-locale`：源语言 locale，例如 `en`。
- `--target-locale`：目标语言 locale，例如 `zh-Hans`。

默认配置位置：

- Windows：`Documents/My Games/Stringer/config.toml`。
- 其他平台：用户配置目录下的 `stringer/config.toml`。

需要隔离用户配置时，可以用 `STRINGER_CONFIG` 指向另一个 `config.toml` 路径；用户知识库会随该配置路径落到同级 `knowledge/` 目录。

配置示例：

```toml
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"

[extraction_filters]
[[extraction_filters.rules]]
id = "pex.identifier_like_source"
enabled = false

[[extraction_filters.rules]]
id = "user.skip_debug_pex_notifications"
enabled = true
reason = "debug notification strings"
when = { all = [
  { field = "kind", op = "eq", value = "pex" },
  { field = "call_member", op = "eq", value = "Notification" },
  { field = "text", op = "regex", value = "^(DEBUG|TODO)" },
] }
```

用户知识库位置固定为默认用户目录下的 `knowledge/`：Windows 为 `Documents/My Games/Stringer/knowledge`，其他平台为用户配置目录下的 `stringer/knowledge`。workspace `stringer.toml` 只读取 `game_release`、`asset_language`、`source_locale` 和 `target_locale`。

`extraction_filters` 只从用户全局配置读取，用于在导出工作区前跳过不需要翻译的提取结果。内置规则也有固定 id，可通过同 id 配置禁用或覆盖；第一版支持 `all`、`any`、`not` 条件树，以及 `eq`、`ne`、`in`、`contains`、`starts_with`、`ends_with`、`regex`、`exists`、`is_empty`、`identifier_like` 和 `tag_list` 操作。

已有 workspace 命令会从 `workspace.json` 读取设置。`knowledge lookup`、`knowledge annotate`、`knowledge validate` 和 `knowledge index rebuild` 默认使用当前目录作为 workspace。

## 知识库

Workspace 知识库默认放在 `<workspace>/knowledge/`：

```text
knowledge/
  terms/
    base.toml
  memory/
    workspace.jsonl
  rules/
    replacements.toml
  index.sqlite
```

知识层加载顺序为：用户全局知识库、workspace 知识库。后加载的层可以覆盖先加载的同 ID 项，覆盖会产生 diagnostic，层名报告为 `workspace`。

术语文件示例：

```toml
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "铁剑"
aliases = ["iron sword"]
case_sensitive = false
status = "preferred"
scope = { game = "SkyrimSe", target_locale = "zh-Hans", kind = "plugin", record_type = "WEAP" }
tags = ["weapon"]
note = "基础游戏武器名。"
```

翻译记忆示例：

```json
{"id":"tm:iron-sword","source":"Iron Sword","target":"铁剑","source_locale":"en","target_locale":"zh-Hans","context":{"kind":"plugin","record_type":"WEAP","subrecord":"FULL"},"quality":"confirmed"}
```

替换规则示例：

```toml
[[rules]]
id = "protect.player_name"
stage = "pre_translate"
mode = "literal"
pattern = "{PLAYER_NAME}"
replacement = "__STRINGER_TOKEN_PLAYER_NAME__"
enabled = false
scope = { kind = ["plugin", "scaleform"] }
note = "预留规则；默认不执行。"
```

重建派生索引：

```powershell
cargo run -p stringer-cli -- knowledge index rebuild `
  --workspace path/to/translations
```

普通 `annotate`、`validate` 和 `lookup` 会优先使用派生索引；索引缺失、过期或损坏时会按层自动重建后再使用。显式运行 `knowledge index rebuild` 适合在导入旧资源或批量编辑知识库后提前刷新全局和 workspace 索引。

### 编辑 Workspace 术语

CLI 可以直接新增、替换或删除 workspace 术语。默认文件是 `<workspace>/knowledge/terms/workspace.toml`；也可以用 `--file` 指向 `<workspace>/knowledge/terms/` 下的其他 `.toml` 文件。`--scope-json` 只接受支持的 scope 键：`game`、`source_locale`、`target_locale`、`kind`、`record_type`、`asset_path`，值为字符串数组。

新增或替换术语：

```powershell
cargo run -p stringer-cli -- knowledge term upsert `
  --workspace path/to/translations `
  --id skyrim.weapon.iron_sword `
  --source "Iron Sword" `
  --target "铁剑" `
  --alias "Iron Blade" `
  --status preferred `
  --scope-json '{"game":["SkyrimSe"],"target_locale":["zh-Hans"],"kind":["plugin"],"record_type":["WEAP"]}' `
  --tag weapon `
  --note "Workspace 固定译名。" `
  --json
```

删除术语：

```powershell
cargo run -p stringer-cli -- knowledge term delete `
  --workspace path/to/translations `
  --id skyrim.weapon.iron_sword `
  --json
```

加上 `--rebuild-index` 会在编辑后立即重建 `<workspace>/knowledge/index.sqlite`。

## 迁移旧翻译资源

`adapt import` 用于把已有翻译资源导入为 Stringer 翻译记忆 JSONL。它不会改模组文件，也不会直接改翻译包；它只读取外部资源，输出可放进 `<workspace>/knowledge/memory/` 的记忆文件，随后由 `knowledge annotate` 和 `knowledge lookup` 使用。

省略 `--out` 时，`adapt import` 会写入标准用户知识库目录下的 `memory/adapt/`。

支持格式：

- `eet`：ESP-ESM Translator / EET 二进制表。
- `eet-xml`：EET XML 导出。
- `eet-json`：EET JSON / DDS 风格导出。
- `xt-sst`：xTranslator `.sst` 文件。

示例：

```powershell
cargo run -p stringer-cli -- adapt import `
  --format xt-sst `
  --input path/to/old-translation.sst `
  --out path/to/translations/knowledge/memory/old-translation.jsonl `
  --source-locale en `
  --target-locale zh-Hans `
  --game SkyrimSe
```

输出行会包含 `id`、`source`、`target`、`source_locale`、`target_locale`、`context`、`origin` 和 `quality`。`context` 会尽量保留记录类型、子记录、Form ID、字符串 ID、字段索引等信息；`origin` 保存来源格式、行号、版本、状态等追踪信息。

质量字段会转换为 Stringer 翻译记忆质量：

- `confirmed`：EET 完成状态、xTranslator 锁定或验证状态。
- `machine`：EET 机器翻译状态。
- `rejected`：EET 拒绝状态。
- `imported`：其他可用旧译文。

空 source 或空 target 会被跳过并计入 diagnostics。导入后建议重建索引：

```powershell
cargo run -p stringer-cli -- knowledge index rebuild `
  --workspace path/to/translations
```

## CLI 速查

查看总帮助：

```powershell
cargo run -p stringer-cli -- --help
```

查看子命令帮助：

```powershell
cargo run -p stringer-cli -- workspace open --help
cargo run -p stringer-cli -- workspace finalize --help
cargo run -p stringer-cli -- workspace upgrade --help
cargo run -p stringer-cli -- adapt import --help
cargo run -p stringer-cli -- knowledge annotate --help
cargo run -p stringer-cli -- knowledge validate --help
cargo run -p stringer-cli -- knowledge lookup --help
cargo run -p stringer-cli -- knowledge index rebuild --help
cargo run -p stringer-cli -- knowledge term upsert --help
cargo run -p stringer-cli -- knowledge term delete --help
```

常用命令：

- `workspace open`：从只读 source root 扫描资产，打开翻译工作区。
- `workspace finalize`：读取翻译工作区，把译文写到输出目录。
- `workspace upgrade`：当前仅报告旧 `manifest.json` 工作区迁移未实现；需要重建旧工作区。
- `workspace inspect`：只读查看 entry files、条目、batch 和 diagnostics，默认输出 JSON。
- `adapt import`：把 EET、EET XML、EET JSON 或 xTranslator SST 转成翻译记忆 JSONL。
- `knowledge annotate`：给翻译工作区写入术语、记忆和知识提示，默认自动填充高置信记忆；需要只写提示时加 `--skip-fill-memory`。
- `knowledge validate`：重算诊断信息，检查术语、禁用译法、占位符、空译文等风险。
- `knowledge lookup`：查询单条文本的提示和诊断；加 `--json` 适合 Agent 读取。
- `knowledge index rebuild`：重建 `<workspace>/knowledge/index.sqlite`。
- `knowledge term upsert/delete`：新增、替换或删除 workspace 术语；可选 `--rebuild-index` 同步刷新索引。

## MCP

Stringer 也提供本地 stdio MCP server，供支持 MCP 的 Agent 使用。MCP tools 覆盖已实现的 CLI 工作流，并返回结构化 JSON 结果。

```powershell
cargo run -p stringer-mcp -- serve
```

术语编辑对应 MCP tools：

- `workspace_inspect_files`、`workspace_inspect_entries`、`workspace_inspect_entry`、`workspace_inspect_batch`、`workspace_inspect_diagnostics`：只读查看工作区，不直接暴露原始 JSONL 文件操作。

- `knowledge_term_upsert`：参数包含可选 `workspace`、可选 `file`、`terms` 和 `rebuild_index`；单条更新也使用一个元素的 `terms` 数组。
- `knowledge_term_delete`：参数包含可选 `workspace`、可选 `file`、`id` 和 `rebuild_index`。

`terms[].status` 使用 `preferred`、`allowed` 或 `forbidden`；`terms[].scope` 使用对象加字符串数组值，例如 `{ "game": ["SkyrimSe"], "kind": ["plugin"] }`。

## Agent Skill

Agent workflow guidance lives in `skills/stringer-workflows/`. Use that Skill for batch translation, subagent splitting, review, validation, and knowledge lookup workflows instead of relying on long CLI help text.

## Workspace 布局

- `crates/stringer-adapt`：旧翻译资源到翻译记忆的转换。
- `crates/stringer-app`：CLI 和 MCP 共用的应用服务层。
- `crates/stringer-cli`：命令行薄入口。
- `crates/stringer-core`：共享文件、语言、诊断和字符串条目模型。
- `crates/stringer-knowledge`：术语、翻译记忆、规则、lookup 和派生索引。
- `crates/stringer-mcp`：本地 stdio MCP server，面向 Agent 暴露结构化 tools。
- `crates/stringer-pex`：PEX 字符串读写。
- `crates/stringer-pipeline`：术语、记忆、规则和诊断处理管线。
- `crates/stringer-plugin`：Bethesda plugin 和 STRINGS 读写。
- `crates/stringer-reader`：扫描模组目录、BSA/BA2 归档和 loose text assets。
- `crates/stringer-scaleform`：Scaleform 翻译表读写。
- `crates/stringer-workspace-api`：工作区生命周期 API 和对外 Rust facade。
- `crates/stringer-workspace-core`：workspace package、settings、lock 和共享基础设施。
- `crates/stringer-workspace-ops`：workspace inspect 和 batch 操作。
- `skills`：项目内 Agent 工作流 Skill。
- `xtask`：维护脚本，例如行数预算检查。
- `docs`：设计和调研文档。
