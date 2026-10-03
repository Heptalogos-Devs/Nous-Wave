# Retrieval 与媒体观测

## 2026-10-02 文本检索

测量使用 6 个文本来源、105 个语义单元和 40 条 grounded query。controlled 与 end-to-end 各使用固定 Authority；end-to-end 完成 105 次 formation。四个 track/variant 组合各执行 40 条查询。

| Track/variant | Recall@1 | Recall@5 | Recall@10 | MRR | p50/p95 ms |
| --- | --- | --- | --- | --- | --- |
| controlled/baseline | .85 | .975 | 1 | .90625 | 252.0808/415.0156 |
| controlled/model-rerank | 1 | 1 | 1 | 1 | 393.1584/468.2948 |
| end-to-end/baseline | .9 | 1 | 1 | .945833333333333 | 238.5688/413.0121 |
| end-to-end/model-rerank | .975 | 1 | 1 | .9875 | 375.6403/567.7227 |

formation coverage 与 provenance precision 均为 1；wrong-source、wrong-entity、stale leakage、empty-result 均为 0。cost unknown。指标对应本页所列语料与 oracle。Wave/combined 缺少真实 topology signal，未测量。2026-10-01 初始测量保存在 [observed-results.json](corpus/observed-results.json)。

两个 NASA WAV 的人工回听显示相近的白噪音/电流声与周期提示音，没有可辨语音，设备身份未知。模型遗漏或否认底噪。这两个样本作为 negative controls 保存在 [audio-negative-controls.json](corpus/audio-negative-controls.json)。

代表音频使用 Kevin MacLeod《Carefree》《Gymnopedie No.1》的 CC-BY-4.0 署名音乐片段。六个三策略流程中五个提交，两个 public recall 成功；direct structured 遗漏 Carefree 的 ukulele/guitar 信息，Gymnopedie 两阶段结构化提出无输入的 visual 内容而被拒绝，已提交 AudioDescription 保留。oracle 支持作者 metadata 中的音乐/乐器族；精确时长、节奏及细节听辨仍未测量。

RAGFlow 是 optional provider，当前没有可用于实验的用户实例/dataset。Topology lane 当前实现 weighted PCA、residual、bounded propagation 与 node-potential；完整 VCP 尚无实现。当前语料没有 topology signal，Wave/combined 未测量。VCPToolBox 源码许可为 CC-BY-NC-SA，当前 MIT package 不分发其源码。


## 2026-10-04 真实 New API 验证

本轮使用本机 New API 与原有 credential，生产 Core/官方 Client 路径经过同一个 research gateway。结构化角色的实际 request Prompt/provider schema digest 与当前 registry 对照，七个角色均有通过同一 Zod owner 的真实输出。credential 扫描未发现落盘 token，trace 文件权限为 0600。模型别名不证明上游实际 weights revision。

| 实际配置 | 使用方式与结果 |
| --- | --- |
| doubao-seed-2.0-mini / openai-chat | SDK text/image、raw audio/video、七个 structured roles，通过合同与角色语义检查 |
| doubao-embedding-vision / openai-embeddings | 2048 维向量被生产链路接受；New API 实际 batch 上限 10，修正旧本地配置的 64 |
| qwen3.7-text-rerank / rerank-v1 | 五条个人语料查询与媒体 recall 实际 rerank 成功 |

音频模式为 direct，视频为 direct；speech_transcription 和 openai-responses 本轮未使用，不建立额外 profile 来填协议矩阵。paid model/credential 不进入 CI。

### 语料与策略

新的代表性输入为 NASA 主持人与 Anil Menon 的 40 秒访谈、传统歌曲 Riddle song 的 25 秒 vocals 片段、Anil Menon 太空行走图片，以及 NASA Apollo 11 档案 3390–3414 秒的点火/上升视频。访谈原候选 Randy Olson 与 Wiki Unseen 片段没有满足所需的清晰多说话人轮次，未采用。访谈以 NASA 已发表 speaker transcript 和独立本地 ASR 为 oracle；音乐不复制歌词；图片/视频通过独立可视检查与选定视频的独立 ASR 核对。精确来源、权利、checksum 和 preprocessing 由 [media.json](corpus/media.json) 拥有。

四个样本均执行 description_only、direct_structured、describe_then_structure，并经过官方 formation/embedding/query。派生来源图均回到实际输入 Artifact。对话轮次、vocals/music、图片动作与环境、视频可见运动在受影响 case 修复后保留；两阶段只访问已提交描述。Apple Writing Tools 旧截图也完成更新后 direct structured 邻近检查。原 corporate corpus、五张 Apple 截图、两首 instrumental 和两段 NASA 音频 negative controls 保留；negative control 没有可辨语音不构成代表性语义缺陷。

### 实际修正

- Structured Contract Registry 成为实际 invocation 的唯一 Zod 选择点；formation/steward inline schema 移入 owner。Metadata 解释 field/catalog/time 语义，raw media 使用 canonical provider contract identity。
- Material summary 持有 supports；basis 与 certainty 分开；原始 source_text 与 embedded_text、description/transcript transport 分开。非法 modality 和目录选择由生产验证器拒绝。
- Prompt family 与 invocation envelope 明确证据来源。Formation 的 evidenceText 替代误命名的 producerMetadata，semanticRole 是短标签；博客 case 的超长语义句被拒绝后，修订 Prompt 重跑成功。
- Vocal direct structuring 曾把可听词句归入 source_text，被拒绝；明确音频词句和原始文本源边界后重跑成功。视频直接结构化曾只列播报里程碑、遗漏可见烟火，并把播报中的人名当说话者；修订后保留烟火/上升，使用未知说话者标签。
- Steward 合同收窄为无任务上下文的忠实压缩。Live 检查发现 contribution batch 总是空文本；Kernel 复用现有 ContextResolver 提供受预算与可访问性约束的 Memory 正文、revision 和 evidence。定向回归及含真实正文的 Steward 调用通过。
- Consolidation 曾混用 source/candidate/support/entity selectors、重复已有 Memory。Prompt 与 schema metadata 指明各目录 key 的来源，并明确不变候选应 skip。修订后的真实 Journal plan proposal 通过 canonical Zod 和 production mapper；未提供的 relation/entity key 继续被拒绝。

### 个人与纵向链路

[personal.json](corpus/personal.json) 选取 Simon Willison 的六篇博客，覆盖 2002 年博客起点、2022 年项目与写作建议、2024 年 link blog、2026 年 beats 与 Datasette blog。六篇均通过官方路径形成 Memory，区分作者/项目实体、出版时间与 Subject 观察时间；源指令样式控制被保留为引用证据，没有执行其中伪造实体或改写事实的指令。

五条 [个人查询](corpus/personal-queries.json) 的 acceptable source 首位命中，wrong-source、wrong-entity、stale 为 0。年代与跨文档案例验证召回所需文档，不声称 Query 已生成跨文档答案。缺失 salary case 返回相关材料但未引入无支持的 salary claim；当前 API 不生成回答或拒答，因此回答层的 refusal 能力未测量。

实际多 Session experience 提交六成员有序 Episode partition 和逐点带支持的 Journal，并运行 Memory/Schema consolidation。分段按文档主题变化解释边界，外部作者的生活未成为 Subject 自传。已有 Memory 的 case 返回 skip；另一个 Journal plan 提出了带 exact selector 的修订。提案检查使用现有 research:longitudinal runner 和 production mapper。小样本未形成边界精度或广泛模型质量统计，也未证明 Schema 泛化在更复杂轨迹上的稳定性。

### 可复查位置与剩余范围

本轮 ignored run root 为 `data/research/runs/real-new-api-2026-10-04/`，逐次 wire capture 位于 `traces/`；当前合同示例为 `data/research/inspection/model-contracts-current/memory_formation/`，video override 示例为 `data/research/inspection/video-description/`。检查命令由 [开发脚本](../../scripts/README.md#模型合同与-trace-检查) 拥有。旧 trace 保留其原 Prompt/schema 身份，不伪称当前匹配。

本轮执行 `just check-fast`、`just check`、`just smoke`，均通过。`just audit` 完成 cargo-deny、cargo-shear、TypeScript 结构审计后，在 cargo-dupes 报告 24 组 exact duplicates（阈值 16）而失败；使用同一工具复查基线 65533c0 也是 24 组，未调整阈值或启动跨 owner 去重工程。其后的 OSV source scan 单独执行，无发现。paid live 保持独立于默认 CI。

本轮支持继续 Subject Cognition 开发；模型输出仍须 owner 校验。待决范围为别名背后的实际 weights revision、更大个人轨迹的语义稳定性、精细分段边界和 Schema 泛化质量。Self、Social Cognition、Motivation、真实 topology/Wave 与 RAGFlow 不在本轮实施范围。

[返回文档目录](../INDEX.md)
