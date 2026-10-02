# Reference Consumer — `apps/nous-cli`

## Purpose

创建一个可以真正手工使用 Nous Wave 的第一方最小consumer。

它支持从公开接口手工使用系统。

## Dependency rule

允许：

```text
@nous-wave/client
@nous-wave/client/node
Node standard library
一个确有价值的小型CLI/rendering依赖
```

禁止 import：

```text
apps/nous-core/src/*
Kernel internal protocol
Rust implementation
persistence
tests/*
scripts/smoke/*
```

## Node Client artifact support

当前 Core已经有authenticated streaming artifact upload HTTP endpoint，但official Node Client没有helper。

在 `@nous-wave/client/node` 加正式artifact API，使 `connectNousInstance()` 返回的consumer可以：

```text
artifacts.uploadFile(...)
artifacts.uploadBytes(...)
```

`uploadFile`必须stream；不要把8GiB允许上限实现成一次`readFile()`。

实际允许上限由 Kernel `[bootstrap.object_store].max_upload_bytes` 单独拥有。Core 与 Node Client 通过 typed `MaterialService.GetLimits` / private Kernel equivalent 读取有效值；不得各自再声明一份部署上限。

认证沿用Core bearer；token不得暴露给CLI output。

## CLI connection

root提供：

```text
pnpm nous -- ...
```

或等价workspace command。

launcher解析profile并传入RunRoot discovery；consumer不推测DataRoot或源码位置。

```text
nous status
nous --locator <bootstrap.toml> status
```

CLI可以保存本地current subject/session/work-context选择，这些state不是Nous Authority。

## Commands

### Subject

```text
nous subject create
nous subject list
nous subject use <id>
```

### Session

```text
nous session open
nous session show
nous session close
```

### Observe

```text
nous observe text --text "..."
nous observe file <path>
```

file path：

```text
stream upload Artifact
→ record Observation referencing artifact
→ print artifact / source-region / occurrence IDs
```

### Derive

```text
nous derive <source-region-id>
nous derive <source-region-id> --strategy description_only
nous derive <source-region-id> --strategy direct_structured
nous derive <source-region-id> --strategy describe_then_structure
```

输出：

- committed representation IDs；
- kind；
- selected representation；
- model/profile/protocol redacted provenance；
- degradation。

### Form

```text
nous form <occurrence-id>
nous form <occurrence-id> --representation <id>
```

必须调用public ModelService formation。

### Embedding

```text
nous embeddings prepare
```

以bounded batch循环，直到：

- committed=0；
- degradation；
- caller max batch budget达到。

### Query

核心：

```text
nous query '<NousQL>'
```

原样通过public NousQL path。

人类输出至少包含：

- status；
- bound query；
- final rank；
- lexical ref / canonical ref；
- text；
- evidence families；
- baseline/rerank/final score（可得时）；
- degradation；
- lane/drop diagnostics摘要。

`--json` 输出稳定machine-readable结构。

### Trace

```text
nous trace <lexical-ref-or-canonical-ref>
```

Memory trace至少沿：

```text
Memory object
→ revision
→ formation producer
→ supports
→ occurrence
→ source region
→ derived representation chain
→ producer signatures
→ original Artifact/source
```

如果现有public services不足，增加正式public trace/read surface；禁止CLI直查PostgreSQL。

### WorkContext

只保留最小：

```text
nous context create
nous context foreground
nous context show
nous context end
```

不为了CLI复刻完整产品。
