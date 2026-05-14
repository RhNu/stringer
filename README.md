# Stringer

Stringer 是一个面向 Bethesda 模组本地化的命令行工具和 Rust workspace。它把可翻译文本从模组目录导出成适合人工或 Agent 编辑的翻译包，再把译文写回到覆盖目录；同时提供术语、翻译记忆、规则校验和单条查询能力，帮助 Agent 在没有额外说明的情况下判断怎么翻译、怎么检查、怎么导入。

当前主要覆盖：

- Plugin 本地化文本：`.esp`、`.esm`、`.esl` 以及配套 `.strings`、`.dlstrings`、`.ilstrings`。
- PEX 脚本字符串：`.pex`。
- Scaleform 翻译表：`Interface/Translations/*.txt`。
- 知识层：术语表、翻译记忆、替换规则、派生 SQLite 索引。

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

推荐 Agent 使用完整参数调用，避免依赖本机默认配置。

1. 从模组目录导出翻译包：

```powershell
cargo run -p stringer-cli -- export `
  --root path/to/mod-root `
  --out path/to/translations `
  --game-release SkyrimSe `
  --asset-language English `
  --source-locale en `
  --target-locale zh-Hans
```

2. 用知识库给翻译包写入提示：

```powershell
cargo run -p stringer-cli -- knowledge annotate `
  --root path/to/mod-root `
  --translations path/to/translations
```

3. 编辑 `path/to/translations/entries/**/*.jsonl`。每行是一个翻译记录，通常只需要填写或修改 `translation`：

```json
{"id":"plugin:Example.esp:WEAP:0x00001234:FULL:0","source":"Iron Sword","translation":"铁剑","context":{"record_type":"WEAP","subrecord":"FULL"}}
```

4. 校验翻译包：

```powershell
cargo run -p stringer-cli -- knowledge validate `
  --root path/to/mod-root `
  --translations path/to/translations
```

5. 写回覆盖目录：

```powershell
cargo run -p stringer-cli -- import `
  --root path/to/mod-root `
  --translations path/to/translations `
  --override-root path/to/output-override
```

`import` 会拒绝把覆盖目录写到源模组目录内部。建议把 `override-root` 指向新的空目录，再由模组管理器加载。

## 翻译包结构

`export` 输出的是一个目录，不是单个文件。

```text
translations/
  manifest.json
  entries/
    plugin/<asset>/<record_type>.jsonl
    pex/<asset>.jsonl
    scaleform/<asset>.jsonl
```

`manifest.json` 记录 schema、游戏版本、资产语言、locale 和条目文件列表。`entries/**/*.jsonl` 每行一个记录，常见字段如下：

- `id`：稳定条目 ID，导入时用它定位源文本。
- `source`：源文本，不建议改。
- `translation`：译文；缺失时导入会跳过该条。
- `context`：记录类型、子记录、Form ID、Scaleform key、PEX 调用位置等上下文。
- `hints`：`knowledge annotate` 写入的术语和记忆提示。
- `diagnostics`：`knowledge validate` 写入的校验结果。

`import` 只读取 `id` 和 `translation`。`hints`、`diagnostics` 和其他扩展字段不会影响写回。

## 配置

CLI 支持从默认配置文件读取基础设置，也支持命令行覆盖。为了让 Agent 自洽，推荐显式传入这些参数：

- `--game-release`：`SkyrimLe` 或 `SkyrimSe`。
- `--asset-language`：Bethesda 资产语言，例如 `English`、`Chinese`、`ChineseSimplified`。
- `--source-locale`：源语言 locale，例如 `en`。
- `--target-locale`：目标语言 locale，例如 `zh-Hans`。

默认配置位置：

- Windows：`Documents/My Games/Stringer/config.toml`。
- 其他平台：用户配置目录下的 `stringer/config.toml`。

配置示例：

```toml
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"

[knowledge]
global_root = "knowledge"
```

`knowledge lookup` 和 `knowledge index rebuild` 也会尝试读取模组根目录下的 `stringer.toml`。`export` 当前只读取默认配置和命令行覆盖参数；如果需要可复现的 Agent 调用，请直接传入四个设置参数。

## 知识库

项目知识库默认放在模组根目录的 `knowledge/`：

```text
knowledge/
  terms/
    base.toml
  memory/
    project.jsonl
  rules/
    replacements.toml
.stringer/
  indexes/
    knowledge.sqlite
```

知识层加载顺序为：内置默认值、全局知识库、游戏/语言库、项目知识库、命令行 override。后加载的层可以覆盖先加载的同 ID 项，覆盖会产生 diagnostic。

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
  --root path/to/mod-root `
  --game-release SkyrimSe `
  --asset-language English `
  --source-locale en `
  --target-locale zh-Hans
```

普通 `annotate`、`validate` 和 `lookup` 会优先使用新鲜索引；索引缺失或过期时会回退到文件知识库，并报告 `knowledge.index_stale`。

## CLI 速查

查看总帮助：

```powershell
cargo run -p stringer-cli -- --help
```

查看子命令帮助：

```powershell
cargo run -p stringer-cli -- export --help
cargo run -p stringer-cli -- knowledge annotate --help
cargo run -p stringer-cli -- knowledge validate --help
cargo run -p stringer-cli -- knowledge lookup --help
cargo run -p stringer-cli -- knowledge index rebuild --help
```

常用命令：

- `export`：扫描模组根目录，导出翻译包。
- `knowledge annotate`：给翻译包写入术语、记忆和知识提示，可选自动填充高置信记忆。
- `knowledge validate`：重算诊断信息，检查术语、禁用译法、占位符、空译文等风险。
- `knowledge lookup`：查询单条文本的提示和诊断；加 `--json` 适合 Agent 读取。
- `knowledge index rebuild`：重建 `.stringer/indexes/knowledge.sqlite`。
- `import`：读取翻译包，把译文写到覆盖目录。

## Workspace 布局

- `crates/stringer-core`：共享文件、语言和字符串条目模型。
- `crates/stringer-plugin`：Bethesda plugin 和 STRINGS 读写。
- `crates/stringer-pex`：PEX 字符串读写。
- `crates/stringer-scaleform`：Scaleform 翻译表读写。
- `crates/stringer-pipeline`：术语、记忆、规则和诊断管线。
- `crates/stringer-workspace`：工作区 API、翻译包、知识层和导入导出流程。
- `crates/stringer-cli`：命令行薄入口。
- `xtask`：维护脚本，例如行数预算检查。
- `docs`：设计和调研文档。
