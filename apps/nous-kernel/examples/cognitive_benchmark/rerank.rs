use super::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub(super) struct Bridge {
    _child: Child,
    binding: serde_json::Value,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
}
impl Bridge {
    pub(super) async fn open() -> Result<Option<Self>> {
        let Some(config) = std::env::var_os("NOUS_RESEARCH_RERANK_CONFIG") else {
            return Ok(None);
        };
        let locator = std::env::var_os("NOUS_RESEARCH_RERANK_LOCATOR")
            .ok_or_else(|| Error::Invalid("rerank locator required".into()))?;
        let mut child = Command::new("corepack")
            .args([
                "pnpm",
                "exec",
                "tsx",
                "scripts/research/cognitive-rerank.ts",
                "--config",
            ])
            .arg(config)
            .arg("--locator")
            .arg(locator)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(failure)?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| Error::Internal("rerank stdin absent".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Internal("rerank stdout absent".into()))?;
        let mut bridge = Self {
            _child: child,
            binding: serde_json::Value::Null,
            input,
            output: BufReader::new(stdout).lines(),
        };
        let ready = bridge.read().await?;
        if ready["ready"] != true {
            return Err(Error::Unavailable("rerank bridge not ready".into()));
        }
        bridge.binding = ready;
        Ok(Some(bridge))
    }
    async fn read(&mut self) -> Result<serde_json::Value> {
        let line =
            tokio::time::timeout(std::time::Duration::from_secs(310), self.output.next_line())
                .await
                .map_err(|_| Error::Unavailable("rerank bridge timed out".into()))?
                .map_err(failure)?
                .ok_or_else(|| Error::Unavailable("rerank bridge ended".into()))?;
        serde_json::from_str(&line).map_err(failure)
    }
    async fn rank(&mut self, query: &str, hits: &[CognitiveHit]) -> Result<serde_json::Value> {
        let docs: Vec<_> = hits
            .iter()
            .map(|hit| hit.representation.as_deref().unwrap_or(""))
            .collect();
        let mut request = serde_json::to_vec(&serde_json::json!({"query":query,"documents":docs}))
            .map_err(failure)?;
        request.push(b'\n');
        self.input.write_all(&request).await.map_err(failure)?;
        self.input.flush().await.map_err(failure)?;
        self.read().await
    }
}

pub(super) async fn apply(
    runtime: &NousRuntime,
    subject: SubjectId,
    text: &str,
    execution: nous_runtime::QueryExecution,
    bridge: &mut Bridge,
) -> Result<(CognitiveQueryResult, serde_json::Value)> {
    let (pool, ticket) = runtime.cognition.retain_query(execution)?;
    let ticket =
        ticket.ok_or_else(|| Error::Unavailable("rerank validation snapshot absent".into()))?;
    let candidates: Vec<_> = pool
        .results
        .iter()
        .filter(|hit| {
            hit.representation
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
        })
        .cloned()
        .collect();
    let mut reply = if candidates.len() >= 2 {
        match bridge.rank(text, &candidates).await {
            Ok(value) => value,
            Err(error) => {
                runtime.cognition.release_query(subject, ticket)?;
                return Err(error);
            }
        }
    } else {
        serde_json::json!({"order":[],"error":null,"skipped":"fewer_than_two_candidates"})
    };
    reply["binding"] = bridge.binding.clone();
    #[derive(Deserialize)]
    struct Ranked {
        index: usize,
        relevance_score: f64,
    }
    let rankings: Vec<Ranked> = serde_json::from_value(reply["order"].clone()).map_err(failure)?;
    let order = rankings
        .into_iter()
        .map(|item| {
            let hit = candidates
                .get(item.index)
                .ok_or_else(|| Error::Invalid("rerank index outside validated pool".into()))?;
            Ok((hit.reference.clone(), item.relevance_score))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut result = runtime
        .cognition
        .finalize_query(
            subject,
            ticket,
            order,
            Vec::new(),
            nous_runtime::CognitiveContributors {
                shared: Some(&runtime.serving),
                memory: runtime
                    .memory
                    .as_ref()
                    .map(|m| m as &dyn nous_runtime::CognitiveContributor),
            },
        )
        .await?;
    if !reply["error"].is_null() {
        result.degradation.push(Degradation {
            code: "query_rerank_unavailable".into(),
            detail: Some("Validated baseline retained after production rerank failure".into()),
        });
        result.status = QueryStatus::Degraded;
    }
    Ok((result, reply))
}
