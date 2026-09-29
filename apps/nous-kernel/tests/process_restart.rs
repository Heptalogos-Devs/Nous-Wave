#[path = "test_support/mod.rs"]
mod test_support;

use nous_protocol::{kernel, public as p};
use std::{env, path::Path, process::Stdio};
use tempfile::TempDir;
use test_support::database;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};
use tonic::{client::Grpc, codegen::http::uri::PathAndQuery};
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
    let text = "\n\n[bootstrap.database]\nmode = \"external\"\nurl = \"\"\nmax_connections = 4\nname = \"restart\"\ninstall_dir = \"postgres-install\"\ndata_dir = \"postgres-data\"\n\n[bootstrap.object_store]\nbackend = \"fs\"\nroot = \"process-objects\"\nmax_upload_bytes = 1048576\n\n[bootstrap.serving]\nroot = \"process-serving\"\n\n[settings.capabilities.process]\nmemory = true\n\n[settings.capabilities.subject_defaults]\nmemory = true\n".to_string();
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
    let endpoint: String = serde_json::from_str::<serde_json::Value>(&line)
        .expect("Kernel discovery JSON")["endpoint"]
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
async fn kernel_process_restart_restores_subject_session_and_runtime_checkpoint() {
    let (root, postgres_url, _postgres) = database().await;
    let kernel_config = config(&root);
    let subject_id = Uuid::from_u128(8801).to_string();
    let operation_id = Uuid::from_u128(8802).to_string();

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
            operation_id,
            capabilities: Some(p::SubjectCapabilities { memory: true }),
        },
    )
    .await
    .expect("create Subject through Kernel RPC");
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
    .expect("open Session through Kernel RPC");
    let session_id = session.session_id.clone();
    let mutation: kernel::MutateRuntimeResponse = unary(
        channel,
        &token,
        "/nous.wave.kernel.v1alpha1.RuntimeStoreService/MutateRuntime",
        kernel::MutateRuntimeRequest {
            subject_id: subject_id.clone(),
            session_id: session_id.clone(),
            expected_runtime_revision: 0,
            checkpoints: vec![kernel::Checkpoint {
                owner_kind: "focus".into(),
                owner_key: "process-focus".into(),
                schema_version: 1,
                revision: 0,
                payload: vec![8, 8, 0, 1],
            }],
            change_foreground: true,
            foreground_key: Some("process-focus".into()),
        },
    )
    .await
    .expect("persist runtime checkpoint");
    assert_eq!(mutation.runtime_revision, 1);
    stop_kernel(child).await;

    let (child, token, channel) = start_kernel(&kernel_config, &postgres_url).await;
    let restored_subject: p::Subject = unary(
        channel.clone(),
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/GetSubject",
        p::SubjectRequest {
            subject_id: subject_id.clone(),
        },
    )
    .await
    .expect("restore Subject through second Kernel process");
    assert_eq!(restored_subject.subject_id, subject_id);
    let restored_session: p::Session = unary(
        channel.clone(),
        &token,
        "/nous.wave.kernel.v1alpha1.AuthorityService/GetSession",
        p::ObjectRequest {
            subject_id: subject_id.clone(),
            id: session_id.clone(),
        },
    )
    .await
    .expect("restore Session through second Kernel process");
    assert_eq!(restored_session.runtime_revision, 1);
    assert_eq!(
        restored_session.active_focus_id.as_deref(),
        Some("process-focus")
    );
    let snapshot: kernel::ReadRuntimeResponse = unary(
        channel,
        &token,
        "/nous.wave.kernel.v1alpha1.RuntimeStoreService/ReadRuntime",
        kernel::ReadRuntimeRequest {
            subject_id,
            session_id,
            owner_kind: "focus".into(),
        },
    )
    .await
    .expect("restore runtime checkpoint");
    assert_eq!(snapshot.checkpoints.len(), 1);
    assert_eq!(snapshot.checkpoints[0].owner_key, "process-focus");
    assert_eq!(snapshot.checkpoints[0].payload, vec![8, 8, 0, 1]);
    stop_kernel(child).await;
}
