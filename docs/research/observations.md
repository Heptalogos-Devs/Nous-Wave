# Retrieval 与媒体观测

2026-10-02 观测来自6个文本来源、105单元、40条 grounded query，每轨使用固定 Authority；105个 end-to-end formation 完成。四变体共160条查询均 complete。

| Track/variant | Recall@1 | Recall@5 | Recall@10 | MRR | p50/p95 ms |
| --- | --- | --- | --- | --- | --- |
| controlled/baseline | .85 | .975 | 1 | .90625 | 252.0808/415.0156 |
| controlled/model-rerank | 1 | 1 | 1 | 1 | 393.1584/468.2948 |
| end-to-end/baseline | .9 | 1 | 1 | .945833333333333 | 238.5688/413.0121 |
| end-to-end/model-rerank | .975 | 1 | 1 | .9875 | 375.6403/567.7227 |

formation coverage/provenance precision=1，wrong-source/entity、stale leakage、empty rate=0，仅适用于该语料与 oracle。Wave/combined 缺真实 topology signal，未测量；cost unknown。

用户回听两个 NASA WAV：近重复的白噪音/电流声与周期提示音，无可辨语音，设备身份未知。模型遗漏/否认底噪；样本移至 `audio-negative-controls.json`。

代表音频使用 Kevin MacLeod《Carefree》《Gymnopedie No.1》的 CC-BY-4.0 署名音乐片段。六个三策略流程中五个提交，两个 public recall 成功；direct structured 遗漏 Carefree 的 ukulele/guitar 信息，Gymnopedie 两阶段结构化提出无输入的 visual 内容而被拒绝，已提交 AudioDescription 保留。oracle 支持作者 metadata 中的音乐/乐器族；精确时长、节奏及细节听辨仍未测量。

RAGFlow 是 optional provider，用户实例/dataset 尚未配置。完整 VCP route 的公开算法审读不等于当前 topology 实现；当前仅 weighted PCA、residual、bounded propagation/node-potential lane。VCPToolBox 的 CC-BY-NC-SA 源码未并入 MIT payload。

[返回文档目录](../INDEX.md)
