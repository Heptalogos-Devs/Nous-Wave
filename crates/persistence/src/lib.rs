//! PostgreSQL Authority repositories and migration ownership.
mod identity;
mod mutations;
pub use mutations::{
    MutationEnvelope, MutationReceipt, MutationStart, OwnerLock, check_receipt, commit_receipt,
};
mod model_workflow;
pub use model_workflow::{WORKFLOW_VALUE_MAX_BYTES, WorkflowOwner, WorkflowReservation};
mod episode_text;
pub use episode_text::EpisodeTextBudget;
mod producer;
mod projection_input;
mod projections;
mod query_descriptors;
mod references;
pub use identity::{IdentityBinding, lexical_prefix, validate_lexical};
pub use query_descriptors::QueryDescriptor;
mod serving;
mod topology_input;
pub use projection_input::{TextProjectionFragment, TextProjectionInput, TextProjectionSource};
pub use projections::{DenseInvalidation, ProjectionInvalidation};
pub use serving::ServingRecord;
pub use topology_input::{CognitiveProjectionInput, TopologyEdgeSource, TopologyProjectionInput};

use nous_core::{Error, OperationId, Result, SubjectId};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};

/// Serialize a Subject-scoped mutation receipt within its Authority transaction.
pub async fn lock_operation(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("{}:{}", subject.0, operation.0))
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    Ok(())
}

#[derive(Clone)]
pub struct AuthorityStore {
    pool: PgPool,
}

impl AuthorityStore {
    pub async fn active_subjects(&self) -> Result<Vec<SubjectId>> {
        Ok(sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT subject_id FROM subjects WHERE status='active' ORDER BY subject_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(SubjectId)
        .collect())
    }
    pub async fn connect(
        url: &str,
        max_connections: u32,
        acquire_timeout: std::time::Duration,
    ) -> Result<Self> {
        if max_connections == 0 {
            return Err(Error::Invalid("max_connections must be positive".into()));
        }
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(acquire_timeout)
            .connect(url)
            .await
            .map_err(database_error)?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|error| Error::Infrastructure(error.to_string()))
    }

    pub async fn check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    pub async fn subject_exists(&self, subject: SubjectId) -> Result<bool> {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM subjects WHERE subject_id=$1)")
            .bind(subject.0)
            .fetch_one(&self.pool)
            .await
            .map_err(database_error)
    }

    pub async fn authority_seq(&self, subject: SubjectId) -> Result<i64> {
        sqlx::query_scalar::<_, i64>("SELECT authority_seq FROM subjects WHERE subject_id=$1")
            .bind(subject.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(database_error)?
            .ok_or_else(|| Error::NotFound("subject not found".into()))
    }

    pub async fn begin(&self) -> Result<Transaction<'_, Postgres>> {
        self.pool.begin().await.map_err(database_error)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

pub fn database_error(error: sqlx::Error) -> Error {
    match &error {
        sqlx::Error::RowNotFound => Error::NotFound("Authority record not found".into()),
        sqlx::Error::Database(e) if e.is_unique_violation() => Error::Conflict(error.to_string()),
        sqlx::Error::Database(e) if e.is_check_violation() || e.is_foreign_key_violation() => {
            Error::Invalid(error.to_string())
        }
        _ => Error::Infrastructure(error.to_string()),
    }
}
