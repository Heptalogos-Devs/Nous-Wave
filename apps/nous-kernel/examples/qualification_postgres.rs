//! Explicit developer/qualification setup, never part of ordinary runtime boot.
use postgresql_embedded::{PostgreSQL, SettingsBuilder, VersionReq};
use std::{path::PathBuf, process::Command, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let installation = std::env::var_os("NOUS_WAVE_POSTGRES_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/dev/runtime/postgresql")
        });
    let executable = installation.join(if cfg!(windows) {
        "bin/postgres.exe"
    } else {
        "bin/postgres"
    });
    if executable.is_file() {
        let version = Command::new(&executable).arg("--version").output()?;
        if !version.status.success()
            || String::from_utf8(version.stdout)?.split_whitespace().nth(2) != Some("18.6")
        {
            return Err(
                "Qualification PostgreSQL must be version18.6; replace it explicitly".into(),
            );
        }
    } else {
        if installation.exists() {
            return Err("Partial qualification runtime installation; resolve it explicitly".into());
        }
        let parent = installation.parent().ok_or("Runtime parent required")?;
        std::fs::create_dir_all(parent)?;
        let scratch = tempfile::TempDir::new_in(parent)?;
        let settings = SettingsBuilder::new()
            .version(VersionReq::parse("=18.6.0")?)
            .installation_dir(scratch.path().join("install"))
            .data_dir(scratch.path().join("cluster"))
            .password_file(scratch.path().join("postgres.pgpass"))
            .password(uuid::Uuid::now_v7().to_string())
            .timeout(Some(Duration::from_secs(60)))
            .temporary(false)
            .build();
        let mut postgres = PostgreSQL::new(settings);
        postgres.setup().await?;
        let installed = postgres.settings().installation_dir.clone();
        drop(postgres);
        std::fs::rename(installed, &installation)?;
    }
    println!("PASS qualification PostgreSQL18.6 prepared; ordinary serve remains acquisition-free");
    Ok(())
}
