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

[返回文档目录](../INDEX.md)
