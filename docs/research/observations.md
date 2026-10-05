# Retrieval 与媒体观测

[返回文档目录](../INDEX.md)

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

## 2026-10-04 Cognitive Retrieval：协议与配置因果观测

本节记录 PR #14 之后的新实验，保留上节历史数据。本轮 Nous 基线为 d7ae4836；Vault 为 2614d65e；VCP source 为 e03b891d；运行 New API 镜像标注 revision 2035a82a，独立本地源码分支在该 revision 上修改。精确来源/blob 身份由 [source manifest](corpus/cognitive-retrieval-sources.json) 保存，逐次请求与结果留在 ignored `data/research/runs/cognitive-retrieval-2026-10-04/`。

### ASR 与 Responses

New API 的 VolcEngine adapter 原先在标准转写入口返回 `unsupported audio relay mode`。本地独立分支新增 SAUC WebSocket 转换，接受有界 WAV/MP3 multipart bytes，API key 与有效 resource ID 进入专用 headers；本机 Agent Plan channel 显式选择 `/api/v3/plan/sauc/bigmodel_nostream`。24 kHz PCM WAV 在网关规范化到 16 kHz；不要求公共音频 URL。Fake upstream、取消、错误、格式/时长边界与 TTS 回归通过。

Menon 40 秒对话 WAV 的标准端点转写耗时约 7.2 秒。新增同一 NASA 官方节目中 46.28–76.28 秒的主持人介绍 MP3，具有 NASA transcript 和已冻结独立 ASR 对照。两段均经过 Nous production `speech_transcription`，提交 Transcript 与正常 provenance，再执行 formation；对话继续通过 embedding/Query，返回已形成 Memory。Nous 保持 `openai-audio-transcription`，没有供应商专属协议。转写 oracle 只检验文字事实，不要求转写结果承担视觉/声线描述或 diarization。

Responses 初次请求因本机 Advanced Custom channel 没有对应 route 返回 503；补齐 route 后 provider 返回 200，但缺少 `output_text.annotations`，被当前 SDK 拒绝。New API 的 Responses owner 为缺省/null annotation list 补空数组，保留现有 annotations、usage 与 provider extension；focused handler tests 和 affected live 重跑通过。

同一输入、同一 Prompt/schema 比较了六个角色：formation、Steward、Material structuring、Episode partition、Journal、consolidation。Chat 与 Responses 均通过 canonical Zod；partition、Journal support catalog 与 consolidation production mapper 检查通过。此组纵向结果是 proposal 校验，不声称重放提交了旧 Authority plan。额外公开 API formation 在 persisted role profile 切到 Responses 后提交了新 Memory；其 providerClass、role config digest 和 Producer signature 改变，Prompt/schema digest 保持一致。单次 latency 不能作为协议性能排名。

### Mini / Lite 模型对照

按用户建议加入 `doubao-seed-2.1-lite`，以同一 frozen input、Prompt、canonical Zod 与原角色配置对比 Mini，分别保留 chat/Responses 结果。Lite formation 与 Steward 两种协议均通过，人工检查保留计划中的 Soyuz 任务时态；该小样本不能证明总体推理质量排名。Lite chat Material structuring 通过但耗时约 115 秒；Responses 在原 4096 token 预算下只返回 incomplete reasoning、没有完整结构化产物。原 120 秒 timeout 下，Lite chat 的三个长程角色以及 Responses Journal/consolidation 超时；Responses Episode 返回合法 no_change，与 Mini 的主题 partition 不同，缺少 audited segmentation oracle 时不判赢家。

单独 operational variant 延长 timeout 至 300 秒，并将 Material structuring 输出预算提高至 8192，保持输入/Prompt/schema；此结果不混入固定配置对照。延长后的 chat Episode 与 consolidation、Responses Journal 通过；chat Journal 仍在 300 秒超时，Responses Material/consolidation 达到各自输出上限且没有完整产物。研究同时保存 wire usage 与 latency，区分合同拒绝、timeout/length 和已观察到的语义质量。

### 媒体合同修复与视频输入

第二段语音的 structuring 曾因 `state` 被一律解释为视觉 kind 而误拒。Observation 新增独立 `evidence_channel`，区分 visual/audio/source_text；kind 描述事项，channel 描述证据。Canonical Zod、validator、projection、Prompt 和直接 consumer 一起更新。转写中陈述的状态可以使用 audio evidence，仍不能声称观察到视觉事件。Steward 曾把计划中的 Soyuz 任务压缩成正在进行；修订时间/归属忠实度要求后，同一输入的 chat/Responses 重跑保留未来状态。

Apollo liftoff 与 press clip 均实际执行 direct video、frames+ASR 和 frames-only，description/direct structured 的来源 graph 与 owner 校验通过。Press clip 的音轨返回空识别文本，scene 保留明确 degradation；这只说明该选定音轨没有成功取得 transcript，不能泛化为 ASR 缺陷。研究 runner 允许显式声明预期 degradation，并在结果中保留它。

Contact sheet 复核发现原始 frames envelope 只给一组 timestamps 与一组图片，模型曾错配黑白帧的时间，并把音频里的升空事件转成早期帧的可见事实。输入改为每张图片紧邻 frame index/timestamp label，frame envelope 身份进入 preprocessing digest。Frames 的可用 audio 以实际 transcript 为准，音频支持可指向 `T001` 的 committed Transcript；相同可用性传入第二阶段 structuring。Focused regression 拒绝 frames-only 的伪 audio coverage。新 labelled frames-only case 保留烟云、亮光、后期可见 rocket 与时间顺序；自由描述对模糊原因的措辞仍须独立语义评阅。

### 已证明的配置效果与 Wave freeze

- `audio.input_mode`：同一 Menon SourceRegion 从 transcription 改为 direct，实际产物由 Transcript 变为 AudioDescription，providerClass 从标准转写变为 chat，无降级。
- `material.strategy`：同一 inline-text SourceRegion 省略 strategy 参数，配置从 description_only 改为 direct_structured 后，实际产物从 extracted_text 变为 structured_interpretation。
- `video.input_mode`：direct wire 发送原始 video；frames wire 使用 FFmpeg 帧、采样时间与可选独立 ASR，quality/provenance/preprocessing identity 随之变化。
- `video.frame_end_margin_seconds`：0.1→0.9，24 秒样本的末帧实际时间 23.9→23.1；set 后 active 保持不变，restart 后采用 desired。
- `roles.memory_formation`：配置切换到 Responses profile 后，真实 wire 与已提交 Producer identity 改变。
- `retrieval.cognitive.profile`：公开 exact revision query 在 native→baseline→native 切换时，topology diagnostics 为 enabled→skipped→enabled，exact 命中均保留且无降级。`serving.topology.enabled` 的 false active 在 restart 后变为 true，随后 native lane 实际执行。
- `retrieval.query.default_result_limit`：首次公开 NousQL probe 改为 1 仍返回两个命中；owner defect 是 compiler 强行补 12。删除该默认后，省略 `$limit` 的请求按配置返回 1/2，显式 limit 继续保留。

其余本轮 targeted descriptors 仍继续按各 consumer 的真实行为验证，不把 Catalog set/get 当效果证据。

现有 `experimental-node-potential-v1` 的数值 golden 冻结 graph conductance、hub penalty、provenance-root dedup、outbound mass、immediate return、FIR potential、state truncation、discarded mass、排序与空 seed；公开 unavailable-topology degradation 有单独回归。Reference baseline 与浮点 tolerance 随 fixture 保存。请求准备现在持有共享 embedding 与只读 lexical/dense hits；Dense 的多代匹配消费同一 embedding，FORBIDDEN、缺失 generation 与 provider failure 保留原 lane 状态和 Runtime requirement 裁决。Native readout 从不可变 QueryObservation 读取唯一 QueryRiver，观测保存 query/profile/generation/config subset/时间约束和 seeds，不保存候选排名；普通 diagnostics 增加 profile、实际激活边与最深传播 hop。

定向图复现了两个旧 Wave 缺陷：remaining budget 只递减却不限制传播；合流状态保留单个 origin，后续节点丢失其他来源。修复后，不足边成本的状态停止传播，合流携带所有来源，输出 provenance 和等能量 tie 顺序稳定。原 default golden 数值与排序仍通过；预算与多源 regression、共享信号 unit 和真实 PostgreSQL association/query integration 通过。typed profile registry 与仅 cognitive lane 所需 embedding 已接入；真实 PostgreSQL regression 验证冻结旧 plan、baseline lane 省略、profile 切换 generation/config 身份与 mismatch unavailable。VCP parity 和 benchmark utility 仍在本轮后续工作中，不能由这次 native refactor 代替。

## 2026-10-05 核心认知恢复

PR #15 的大型 observatory/garden/archive 模板结果与全量配对 rerank 降级为探索性接线记录；不能据此判定自动 Tag/Association、Accretion、真实情境 recall 或算法胜负。已付费 provider cache 与原始输出保持 ignored 保存，tracked 模板只保留小型结构 fixture。VCP frozen parity 保留关键数学输入/输出、source hash 和 tolerance，完整探索 matrix 保存 ignored。

本轮暂停 LongMemEval/LoCoMo 全量、批量 rerank、重复 embedding。按 Prepared Query 功能集、独立 raw-text compatibility、少量 Agent CLI loop 三条输入条件分别验收；当前尚未完成新功能集，不提供新的质量分数。


### 有界联想解释与共享缓存小试

自然 release 场景现有 cognition 的一个冻结 Prepared Query 在 baseline/native/DTSC/RiverMemo 四个 profile 上复用同一 query vector 与 source cache；本次算法执行 provider、新增 embedding、rerank 均为零。Native 命中可读出 `Tag → Memory → Memory` 两跳激活路径、`tag_attachment`/`elaborates` 关系及支持来源；VCP 数值 readout metadata 经过融合后保留。Full diagnostics 汇集各命中的 readout，复合表达式保留分支 trace。路径是实际激活边上的连通见证，不代表全部势能的因果归属；传播不完整和每边最多 16 条支持的截断明确报告。

Native topology artifact 保存投影证据并以版本 7 重新构建，按边索引支持来源；旧版本 asset 不复用。数值 golden 和真实 PostgreSQL 自动 maintenance→Serving→Tag-only query 回归通过。该单题接线结果尚不能代表完整 Functional Corpus 或算法质量验收。


### Prepared 功能集首轮完整闭合检查

13 个已解析 intent 的 baseline/native 结构轨道完成 26 个 owner-finalized readout：provider、新增 embedding、rerank 均为零，新增 Serving generation 为零，net artifact growth 约 1.99 MiB。所有最终输出遵守 limit 16；32-hit pool 独立保存。按相同 profile 连续处理问题，避免每题重复切换 generation。source/path grader 对完整 15-intent 分母记录 30 项：12 项结构支持、14 项失败、4 项缺失 readout；这不是语义质量分数，也不是完整功能验收。两个缺失自动 Tag、未形成的 incident 关联、部分 source coverage 与 visitor interference 仍需处理。四个语义 profile 的完整集合尚未运行。

自然 routine 已通过正常 consolidation 修订旧 late-night 偏好，并形成疲劳归因和 breakfast 后写作的新偏好 Memory，保留直接 occurrence 与 Journal 支持；一次 180 秒机会内使用一个实际 HTTP 请求，约 108 秒完成。incident 在两个请求内完成一次 Episode organization 与一个 snapshot incident Journal，尚未形成跨系统关联。所有失败、预算截停与 retry 均保留 owner/workflow receipts。

闭合检查定位了关系从句 `experience that informed` 被当作开放指代的误判。guard 区分已命名 cognitive noun 后的常用关系从句连接词，保留真正的 demonstrative/pronoun 拒绝，并补齐 possessive/object pronoun 检查。复合 Episode 和原子 Episode 可以合法共享来源；exact event intent 绑定唯一最小 scope，等粒度冲突仍显式 unresolved。


### VCP 实际激活路径出口

VCP readout 现在附带从实际 gating seed 出发的有界 Sense 连通路径、种子权重、激活 flow、wormhole 标记和可用支持来源。显式 Authority evidence 与实际文档 cooccurrence 分别保留关系名称；无来源的 semantic transition 保持空支持。路径是观察中的连通见证，数值排名保持原样。Native/VCP 共享一个有界路径构造器；cycle、不可达节点、hop 边界回归及 23 项 reference parity 通过。

复用原 29-vector cache 的两题四 profile 小试完成 8 个最终 readout，provider/new embedding/rerank 为零、新增 generation 3、net artifact growth 约 1.14 MiB。与修改前的同题结果顺序完全一致；Native 每题 7 个命中有路径，DTSC 最终命中没有 Sense 路径，RiverMemo 每题 1 个命中有路径。没有把缺失路径修补成关联成功。

routine 本轮正常 consolidation 新增了 Sep 9 的 breakfast 后重复习惯 Memory。下一份 consistency group 含 Schema→Memory 的 `link_relation`，被 owner 原有强 Memory relation 合同拒绝而回滚；发现模型 schema 描述错误允许 Schema endpoint。Prompt/schema 已对齐 Memory-only 合同，Core 提前拒绝 Schema/skip action endpoints，来源关系继续使用 Schema evidence links。该失败保留，未手工注入期望 Memory/Tag。


incident 本批三次 provider 请求完成 snapshot 原因/修复 Memory 与后续 Journal；第三份 consistency group 同样使用 Schema relation endpoint，运行进程启动于合同修复前，owner 拒绝并回滚。新合同需由下一批启动验证。独立 text-only compatibility 选择已人工核对：两个 LongMemEval、两个 LoCoMo、两个 Python 官方真实文本问题，共 13 个唯一来源段落。选择 manifest 保存 locator/hash，原文与执行输入 ignored；未执行检索或 embedding，未扩大外部全集。


修正合同后的 routine 单请求机会实际处理了正常队列中更高优先的 Journal consolidation（约 78 秒），成功形成 Sep 14 重复早餐后写作和 Sep 15 拒绝 visitor 午夜建议的两个 grounded Memory。routine 当前 6 个 Memory；显式复核的旧 need 尚未被该请求执行，不能将此结果当作该 exact need 的 replay 验证。未新增 embedding/rerank，provider 上限 1 在后续请求前截停，普通 retry 和 obsolete 结果保留。
