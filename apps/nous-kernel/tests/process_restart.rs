#[path = "test_support/mod.rs"]
mod test_support;

use nous_protocol::public as p;
use std::{env, path::Path, process::Stdio};
use tempfile::TempDir;
use test_support::database;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};
use tonic::{
    Request,
    client::Grpc,
    codegen::http::uri::PathAndQuery,
    transport::{Channel, Endpoint},
};
use tonic_prost::ProstCodec;
use uuid::Uuid;

fn executable() -> String {
    env::var("CARGO_BIN_EXE_nous-kernel").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("debug")
            .join(if cfg!(windows) {
                "nous-kernel.exe"
            } else {
                "nous-kernel"
            })
            .to_string_lossy()
            .into_owned()
    })
}

fn config(root: &TempDir) -> std::path::PathBuf {
    let path = root.path().join("process-restart-kernel.toml");
    let text = r#"
[bootstrap.database]
mode = "external"
url = ""
max_connections = 4
name = "restart"
install_dir = "postgres-install"
data_dir = "postgres-data"

[bootstrap.object_store]
backend = "fs"
root = "process-objects"
max_upload_bytes = 1048576

[bootstrap.serving]
root = "process-serving"

[settings.capabilities.process]
memory = true

[settings.capabilities.subject_defaults]
memory = true
"#;
    std::fs::write(&path, text).expect("write process restart config");
    path
}

fn authorized<T>(token: &str, value: T) -> Request<T> {
    let mut request = Request::new(value);
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {token}")
            .parse()
            .expect("authorization metadata"),
    );
    request
}

async fn unary<Req, Resp>(
    channel: Channel,
    token: &str,
    path: &'static str,
    value: Req,
) -> Result<Resp, tonic::Status>
where
    Req: prost::Message + Default + 'static,
    Resp: prost::Message + Default + 'static,
{
    let mut grpc = Grpc::new(channel);
    grpc.ready()
        .await
        .map_err(|error| tonic::Status::unknown(error.to_string()))?;
    Ok(grpc
        .unary(
            authorized(token, value),
            PathAndQuery::from_static(path),
            ProstCodec::default(),
        )
        .await?
        .into_inner())
}

async fn start_kernel(config: &Path, postgres_url: &str) -> (Child, String, Channel) {
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned();
    let mut child = Command::new(executable())
        .arg("--config")
        .arg(config)
        .env("NOUS_WAVE_POSTGRES_URL", postgres_url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn Kernel child process");
    child
        .stdin
        .as_mut()
        .expect("Kernel stdin")
        .write_all(format!("{token}\n").as_bytes())
        .await
        .expect("send Kernel bootstrap token");
    let stdout = child.stdout.take().expect("Kernel stdout");
    let mut lines = BufReader::new(stdout).lines();
    let line = tokio::time::timeout(std::time::Duration::from_secs(30), lines.next_line())
        .await
        .expect("Kernel startup timeout")
        .expect("Kernel startup result")
        .expect("Kernel discovery line");
    let endpoint = serde_json::from_str::<serde_json::Value>(&line).expect("Kernel discovery JSON")
        ["endpoint"]
        .as_str()
        .expect("Kernel discovery endpoint")
        .to_owned();
    let channel = Endpoint::from_shared(endpoint)
        .expect("Kernel endpoint")
        .connect()
        .await
        .expect("connect Kernel child");
    (child, token, channel)
}

async fn stop_kernel(mut child: Child) {
    if child.try_wait().expect("Kernel status").is_none() {
        child.kill().await.expect("kill Kernel child");
    }
    child.wait().await.expect("wait Kernel child");
}

#[tokio::test]
async fn kernel_process_restart_restores_subject_session_and_work_context() {
    let (root, postgres_url, _postgres) = database().await;
    let kernel_config = config(&root);
    let subject_id = Uuid::from_u128(8801).to_string();

    let (child, token, channel) = start_kernel(&kernel_config, &postgres_url).await;
    let subject: p::Subject = unary(
        channel.clone(),
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/CreateSubject",
        p::CreateSubjectRequest {
            subject_id: Some(subject_id.clone()),
            cognitive_seed: Some(p::CognitiveSeed {
                text: "schema_version = 1".into(),
                format: nous_subject::COGNITIVE_SEED_FORMAT.into(),
                provenance: None,
            }),
            metadata: None,
            operation_id: Uuid::from_u128(8804).to_string(),
            capabilities: Some(p::SubjectCapabilities { memory: true }),
        },
    )
    .await
    .expect("create Subject");
    assert_eq!(subject.subject_id, subject_id);
    let session: p::Session = unary(
        channel.clone(),
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/OpenSession",
        p::SubjectRequest {
            subject_id: subject_id.clone(),
        },
    )
    .await
    .expect("open Session");
    let context: p::WorkContextResponse = unary(
        channel.clone(),
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/CreateWorkContext",
        p::CreateWorkContextRequest {
            operation_id: Uuid::from_u128(8802).to_string(),
            subject_id: subject_id.clone(),
            purpose: "restart continuity".into(),
            unresolved_questions: vec!["resume".into()],
            constraints: None,
            resume_conditions: vec!["same subject".into()],
            budget_summary: None,
            references: Vec::new(),
        },
    )
    .await
    .expect("create WorkContext");
    let context = context.work_context.expect("WorkContext payload");
    let bound: p::Session = unary(
        channel,
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/SetActiveWorkContext",
        p::SetActiveWorkContextRequest {
            operation_id: Uuid::from_u128(8803).to_string(),
            subject_id: subject_id.clone(),
            session_id: session.session_id.clone(),
            expected_runtime_revision: session.runtime_revision,
            work_context_id: Some(context.work_context_id.clone()),
        },
    )
    .await
    .expect("foreground WorkContext");
    assert_eq!(
        bound.active_work_context_id.as_deref(),
        Some(context.work_context_id.as_str())
    );
    stop_kernel(child).await;

    let (child, token, channel) = start_kernel(&kernel_config, &postgres_url).await;
    let restored: p::Session = unary(
        channel.clone(),
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/GetSession",
        p::ObjectRequest {
            subject_id: subject_id.clone(),
            id: session.session_id,
        },
    )
    .await
    .expect("restore Session");
    assert_eq!(
        restored.active_work_context_id.as_deref(),
        Some(context.work_context_id.as_str())
    );
    let restored_context: p::WorkContextResponse = unary(
        channel,
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/GetWorkContext",
        p::GetWorkContextRequest {
            subject_id,
            work_context_id: context.work_context_id,
        },
    )
    .await
    .expect("restore WorkContext");
    assert_eq!(restored_context.work_context.expect("payload").revision, 1);
    stop_kernel(child).await;
}
