# Configuration Service

## 1. 职责

新增 workspace crate：

```text
crates/configuration-service
package: nous-configuration
```

它是基础服务，负责统一的配置注册与解析。它可以依赖：

- `nous-core`；
- `nous-persistence`；
- `serde` / `serde_json` / `toml` / `blake3` / `arc-swap`；
- `tokio`（持久覆盖操作）。

它**不得依赖**：

- memory-domain/service；
- self-domain/service；
- social-domain/service；
- cognitive-retrieval；
- serving。

领域依赖方向是 `domain/service -> configuration-service`，不反向。

## 2. 配置不是 Service Locator

业务代码不能：

```text
CONFIG.get_f64("retrieval.rrf.k")
```

并在算法中临时读全局状态。

正确路径：

```text
operation boundary
  → ConfigService.snapshot_for_subject(subject)
  → owner 的 resolve_policy(snapshot)
  → typed policy
  → pure/domain algorithm
```

这样一次 operation 不会因为中途配置变化得到混合值。

## 3. 基础类型

```rust
pub enum ConfigExposure {
    Standard,
    Advanced,
    Developer,
}

pub enum ConfigScopePolicy {
    SystemOnly,
    SubjectOverrideAllowed,
}

pub enum ConfigApplyMode {
    Live,
    ServingRebuild,
    NewSubjectsOnly,
    RestartProcess,
}

pub enum ConfigSemanticEffect {
    Operational,
    QueryPolicy,
    ServingProjection,
    AuthorityFormation,
    SubjectProvisioning,
}

pub enum ConfigSource {
    ReferenceDefault,
    DeploymentFile,
    PersistedSystem,
    PersistedSubject,
}
```

系统不变量**不进入 `ConfigExposure`**。它们不是配置。

## 4. Typed ConfigKey

```rust
pub struct ConfigKey<T> {
    path: &'static str,
    _marker: PhantomData<fn() -> T>,
}
```

要求：

- path regex：`^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$`；
- 全进程唯一；
- 创建后长期稳定；
- rename 直接修改当前 pre-production contract，不建立 alias，除非真实兼容义务出现。

`T` 必须：

```text
Clone + Send + Sync + Serialize + DeserializeOwned + 'static
```

## 5. ConfigDescriptor

registry 内部对 heterogeneous key 进行 type erasure，但 descriptor 至少保存：

```text
key
owner
human_description
value_type_name
reference_default_json
exposure
scope_policy
apply_mode
semantic_effect
validator
```

不要引入 JSON Schema generator 依赖。当前类型名、默认值、owner description 与 validator 足以服务代码和未来 UI。

## 6. Registry

```rust
pub struct ConfigRegistryBuilder { ... }
pub struct ConfigRegistry { ... }
```

每个 owner 导出：

```rust
pub fn register_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()>;
```

Kernel 在 startup 时依次注册当前编译进 workspace 的 owner。**即使某领域 process capability disabled，也注册其 descriptor**，这样配置文件可以预先声明以后启用的值，配置 introspection 也完整。

重复 key、非法 default、owner 注册失败直接阻止启动。

registry `finish()` 后 immutable；运行期不注册新 key。未来真正实现插件配置时再设计动态扩展。

## 7. 注册 owner

当前至少：

```text
configuration-service   # capabilities / subject defaults
cognitive-runtime       # runtime policy
memory-service          # accessibility
cognitive-retrieval     # fusion/query budgets
serving                 # serving enablement / topology build policy
self-service            # 当前没有必须可调的 Self semantic knob，可只注册空集合
social-service          # formation/query policy
```

Owner 可以没有 key；不要为了“每层都有配置”制造无意义设置。

## 8. Bootstrap configuration 与 Runtime configuration

当前 `apps/nous-kernel/src/bootstrap.rs` 先读一个大 `Config`，其中混合数据库、对象存储、Memory/Serving/算法参数。拆分为：

### BootstrapConfig

只保留必须在 Configuration Service 打开前知道的内容：

```text
server.bind
server.remote_access
database.mode/url/max_connections/name/install_dir/data_dir
object_store.backend/root/max_upload_bytes
serving.root
```

这些是部署启动参数。`NOUS_WAVE_POSTGRES_URL` 继续只覆盖 bootstrap DB endpoint。

### settings

同一 TOML 中 `[settings]` 下所有叶子由 ConfigRegistry 解析。

例：

```toml
[bootstrap.server]
bind = "127.0.0.1:9090"
remote_access = false

[bootstrap.database]
mode = "managed"
max_connections = 8

[bootstrap.object_store]
backend = "fs"
root = "./data/objects"
max_upload_bytes = 8589934592

[bootstrap.serving]
root = "./data/serving"

[settings.capabilities.process]
memory = true
self_cognition = false
social = false

[settings.capabilities.subject_defaults]
memory = true
self_cognition = false
social = false
```

`settings` 未知 leaf key 直接配置错误。

## 9. 配置来源与优先级

当前只实现四层，确定性顺序：

```text
ReferenceDefault
    < DeploymentFile
    < PersistedSystem
    < PersistedSubject
```

PersistedSubject 仅对 `SubjectOverrideAllowed` key 生效。

不要把大量 algorithm env vars 加成第五套入口。Secret/endpoint 继续属于 bootstrap/secret channel。

## 10. 暴露层级与修改权限

内部写入 API 接收：

```rust
pub enum ConfigActorTier {
    StandardUser,
    AdvancedUser,
    Developer,
}
```

权限：

- StandardUser：只能修改 `exposure=Standard` 且 `SubjectOverrideAllowed` 的自己 Subject override；
- AdvancedUser：可以修改 Standard + Advanced 的自己 Subject override；
- Developer：可以修改所有注册配置的 system override；若 key 允许 subject override，也可管理指定 Subject；
- Developer file `[settings]` 可以提供所有可配置 key 的部署值。

本轮不把这个 actor tier 直接暴露为 unauthenticated gRPC 字段。未来 Management API 从真实授权上下文映射 actor tier。

## 11. 持久化

新增 fresh migration：

```text
0006_configuration.sql
```

表：

```sql
CREATE TABLE configuration_state (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    revision BIGINT NOT NULL
);

CREATE TABLE system_configuration_overrides (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    revision BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE subject_configuration_overrides (
    subject_id UUID NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value JSONB NOT NULL,
    revision BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(subject_id, key)
);
```

`configuration_state` 初始化 revision=1。

配置 descriptor 不持久化；descriptor 来源是当前代码 registry。数据库只保存 overrides。

## 12. 幂等修改

Config mutation 使用现有 operation receipt 机制：

```text
set_system_override
set_subject_override
clear_system_override
clear_subject_override
```

请求包含：

```text
operation_id
key
value?
actor tier / trusted authorization context
subject?
```

规则：

- same operation id + same digest → 返回原结果；
- same id + different digest → Conflict；
- mutation、configuration revision +1 与 receipt 同一事务。

## 13. ConfigSnapshot

```rust
pub struct ConfigSnapshot {
    subject_id: Option<SubjectId>,
    revision: i64,
    registry_digest: ConfigDigest,
    effective_digest: ConfigDigest,
    values: Arc<BTreeMap<ConfigKeyId, ResolvedConfigValue>>,
}

pub struct ResolvedConfigValue {
    json: serde_json::Value,
    source: ConfigSource,
}
```

API：

```text
active_system_snapshot()
active_snapshot_for_subject(subject)
desired_system_snapshot()
desired_snapshot_for_subject(subject)
```

普通运行代码只用 `active_*`。

## 14. RestartProcess 配置

配置修改可以先持久化 desired value，但 `RestartProcess` key 不在当前进程中替换 active value。返回：

```text
ConfigChangeOutcome
- revision
- apply_mode
- pending_restart
- active_digest
- desired_digest
```

进程下次启动时该 persisted override 成为 active。

`Live` / `ServingRebuild` / `NewSubjectsOnly` key 成功提交后更新当前进程 active cache。

## 15. Cache

Service 内部：

- system active/desired snapshot 可以使用 `ArcSwap`；
- subject snapshot 使用 `RwLock<HashMap<SubjectId, Arc<ConfigSnapshot>>>`；
- system/subject override变更后只清理受影响的 cache；
- `snapshot.get(KEY)` 不访问数据库。

本轮按单进程写入模型实现，不增加跨进程 LISTEN/NOTIFY/polling。Server 多进程共享配置热更新以后有真实需求再增加。

## 16. Canonical digest

使用 BLAKE3。

registry digest 输入按 key 排序：

```text
key
value_type_name
reference_default_json canonical form
exposure
scope_policy
apply_mode
semantic_effect
```

Effective digest 输入按 key 排序：

```text
registry_digest
key
resolved canonical JSON value
```

`ConfigSnapshot::digest_for(keys)` 只对指定 key subset 计算 digest。

用途：

- Retrieval query diagnostics → retrieval key subset；
- Serving generation → 影响对应 artifact 的 subset；
- Social formation → language convention formation subset；
- 与无关 DB path、其他领域设置解耦。

## 17. Owner 解析 policy

例如 Memory：

```rust
pub fn resolve_accessibility_policy(snapshot: &ConfigSnapshot) -> Result<AccessibilityPolicy>;
```

Cognitive Retrieval：

```rust
pub fn resolve_retrieval_policy(snapshot: &ConfigSnapshot) -> Result<RetrievalPolicy>;
```

Social：

```rust
pub fn resolve_social_policy(snapshot: &ConfigSnapshot) -> Result<SocialPolicy>;
```

不要让 `AccessibilityPolicy` 自己知道数据库/config service。

## 18. 当前 key catalog

### 基础能力 / Developer / SystemOnly

| Key | Default | Apply | Effect |
| --- | --- | --- | --- |
| `capabilities.process.memory` | `true` | RestartProcess | Operational |
| `capabilities.process.self_cognition` | `false` | RestartProcess | Operational |
| `capabilities.process.social` | `false` | RestartProcess | Operational |
| `capabilities.subject_defaults.memory` | `true` | NewSubjectsOnly | SubjectProvisioning |
| `capabilities.subject_defaults.self_cognition` | `false` | NewSubjectsOnly | SubjectProvisioning |
| `capabilities.subject_defaults.social` | `false` | NewSubjectsOnly | SubjectProvisioning |
| `runtime.resident_limit` | `256` | RestartProcess | Operational |
| `serving.lexical.enabled` | `true` | RestartProcess | Operational |
| `serving.dense.enabled` | `true` | RestartProcess | Operational |
| `serving.topology.enabled` | `true` | RestartProcess | Operational |

### Memory accessibility / Advanced / SubjectOverrideAllowed

| Key | Default | Apply |
| --- | ---: | --- |
| `memory.accessibility.epsilon` | 0.02 | Live |
| `memory.accessibility.tau_days` | 30.0 | Live |
| `memory.accessibility.decay` | 0.5 | Live |
| `memory.accessibility.formation_weight` | 1.0 | Live |
| `memory.accessibility.use_weights.referenced` | 1.0 | Live |
| `memory.accessibility.use_weights.acted_on` | 1.4 | Live |
| `memory.accessibility.use_weights.result_supported` | 1.1 | Live |
| `memory.accessibility.use_weights.result_refuted` | 1.2 | Live |
| `memory.accessibility.use_weights.corrected` | 1.5 | Live |
| `memory.accessibility.use_weights.pinned` | 2.0 | Live |
| `memory.accessibility.normal_threshold` | -0.80 | Live |
| `memory.accessibility.deep_threshold` | -1.60 | Live |

`presented` 固定为 0，**不注册**；“呈现不强化”是语义不变量。

### Retrieval / Developer / SystemOnly

```text
retrieval.rrf.k = 60.0
retrieval.rrf.weights.exact = 4.0
retrieval.rrf.weights.runtime = 2.0
retrieval.rrf.weights.entity = 2.5
retrieval.rrf.weights.lexical = 1.5
retrieval.rrf.weights.dense = 1.5
retrieval.rrf.weights.temporal = 1.0
retrieval.rrf.weights.schema_direct = 1.5
retrieval.rrf.weights.self_direct = 2.0
retrieval.rrf.weights.social_relation_direct = 2.0
retrieval.rrf.weights.language_convention_direct = 2.0
retrieval.rrf.weights.topology_wave = 1.0
```

以及当前 QueryPlan 已使用的 effort multiplier / per-lane max / validation budget 参数。把现有 reference 数值逐项注册，不重新调参。

### Topology / Developer / SystemOnly / ServingRebuild

```text
topology.wave.hub_beta = 0.35
topology.wave.hub_penalty_min = 0.35
topology.wave.hub_penalty_max = 1.25
topology.wave.outbound_budget = 0.90
topology.wave.max_hops = 4
topology.wave.max_states = 4096
topology.wave.max_neighbors_per_node = 32
topology.wave.minimum_state_energy = 0.0001
topology.wave.immediate_return_multiplier = 0.20
topology.wave.initial_budget_steps = 4
topology.wave.normal_edge_cost = 1
topology.wave.fir_gamma = 0.55
```

Edge class quality 与 query-time seed weights 也逐项注册，默认保持当前 reference implementation。

### Social

`Advanced / SubjectOverrideAllowed / Live / AuthorityFormation`：

```text
social.language_convention.repeated_external_use_min = 2
social.language_convention.require_same_actor_after_successful_understanding = true
```

`Advanced / SubjectOverrideAllowed / Live / QueryPolicy`：

```text
social.query.scope_preference = ["dyad", "person", "channel", "group", "community"]
```

形成 LanguageConvention 时记录这组 AuthorityFormation key 的 subset digest。

### Standard

本轮实现 `Standard` 暴露等级及修改权限，但**不为了填满层级发明普通用户设置**。后续真正出现普通用户可安全理解的设置时直接注册为 Standard，不需要再改 Configuration Service 架构。

## 19. 硬编码参数归类规则与施工审计

本轮代码修改前，对以下 owner 的 production source 做一次参数清点：

```text
cognitive-runtime
memory-service
cognitive-retrieval
serving
self-service
social-service（新增）
```

对影响行为的 numeric/enum/list literal 按以下规则处理：

1. **改变后仍保持目标语义，只改变质量、成本、阈值、预算、偏好或算法行为** → 注册 configuration key；
2. **改变后会破坏目标设计不变量** → 保留代码不变量，不注册；
3. **协议/安全/防滥用 hard cap** → 保留 hard cap，除非部署差异确实需要调整；若同时存在用户可调 soft limit，则 soft limit进入配置且不得超过 hard cap；
4. **纯数学常量或格式版本** → 保留代码；
5. **第三方库要求/数据结构实现细节** → 不因“看起来是数字”自动配置化。

Agent 不能把不确定语义藏进新 config knob。若一个 hardcoded 值无法按上述规则明确归类且改变它会影响公开语义，报告 `SPEC_GAP`。

施工结束在完成报告列出：

- 新注册的 policy keys；
- 有意保留的主要 hard caps/invariants；
- 当前 owner 中已无已知“需要研究/调参却必须改源码”的 reference 参数。

## 19. Validation

Registry default 与所有 override 都经过 descriptor validator。跨 key 校验由 owner policy resolver执行。

至少：

- accessibility numeric finite、tau/decay > 0、weights >=0、normal_threshold > deep_threshold；
- RRF k > 0、参与 family weight >0；
- Wave 使用现有 `WaveConfig::validate()` 并校验 class/seed weights finite >=0；
- capability subject default不能请求 process capability false 时“自动纠正”；它只是未来 Subject 默认，Subject create时再验证当前 process capability；
- Social repeated_external_use_min >=2；
- scope_preference 是五个合法 scope 的无重复全排列。

## 20. 暂不实现公共配置 API

本轮完成内部 Configuration Service 的 list/get/set/clear 方法和 descriptor introspection。

不新增 unauthenticated public RPC。Management API 以后接入时，只包装同一个 service，并把授权身份映射成 `ConfigActorTier`。

这样先提供配置能力和层级元数据，不提前锁死产品暴露界面。

## 21. 测试

只测试高价值合同：

1. duplicate key注册失败；
2. unknown file key启动失败；
3. default < file < system < subject precedence；
4. SystemOnly不能写 subject override；
5. Standard/Advanced/Developer tier enforcement；
6. RestartProcess override持久但当前 active不变；
7. Live override下一 operation可见；
8. snapshot中途不变；
9. same mutation id/digest幂等；
10. subset digest只受相关 key影响；
11. representative accessibility/RRF/Wave 配置真正改变对应 deterministic behavior。

不要给每个 key 写一条重复 getter test。
