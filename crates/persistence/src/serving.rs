use crate::{AuthorityStore, database_error as db};
use chrono::{DateTime, Utc};
use nous_core::*;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServingRecord {
    pub generation_id: ServingGenerationId,
    pub subject: SubjectId,
    pub family: String,
    pub space: String,
    pub authority_watermark: i64,
    pub implementation_id: String,
    pub implementation_revision: String,
    pub config_digest: String,
    pub artifact_location: String,
    pub artifact_hash: String,
    pub built_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

fn decode(row: sqlx::postgres::PgRow) -> Result<ServingRecord> {
    Ok(ServingRecord {
        generation_id: ServingGenerationId(row.try_get("generation_id").map_err(db)?),
        subject: SubjectId(row.try_get("subject_id").map_err(db)?),
        family: row.try_get("family").map_err(db)?,
        space: row
            .try_get::<Option<String>, _>("space_signature")
            .map_err(db)?
            .unwrap_or_default(),
        authority_watermark: row.try_get("authority_watermark").map_err(db)?,
        implementation_id: row.try_get("implementation_id").map_err(db)?,
        implementation_revision: row.try_get("implementation_revision").map_err(db)?,
        config_digest: row.try_get("config_digest").map_err(db)?,
        artifact_location: row.try_get("artifact_location").map_err(db)?,
        artifact_hash: row.try_get("artifact_hash").map_err(db)?,
        built_at: row.try_get("built_at").map_err(db)?,
        metadata: row.try_get("metadata").map_err(db)?,
    })
}

impl AuthorityStore {
    pub async fn serving_current(&self, subject: SubjectId) -> Result<Vec<ServingRecord>> {
        sqlx::query("SELECT g.* FROM serving_current c JOIN serving_generations g USING(generation_id) WHERE c.subject_id=$1 AND g.state='ready' ORDER BY c.family,c.space_signature")
            .bind(subject.0).fetch_all(self.pool()).await.map_err(db)?.into_iter().map(decode).collect()
    }

    pub async fn publish_generation(&self, record: ServingRecord) -> Result<ServingRecord> {
        let mut tx = self.begin().await?;
        let key = format!(
            "serving:{}:{}:{}",
            record.subject.0, record.family, record.space
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(key)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let current=sqlx::query("SELECT g.* FROM serving_current c JOIN serving_generations g USING(generation_id) WHERE c.subject_id=$1 AND c.family=$2 AND c.space_signature=$3").bind(record.subject.0).bind(&record.family).bind(&record.space).fetch_optional(&mut *tx).await.map_err(db)?.map(decode).transpose()?;
        if current
            .as_ref()
            .is_some_and(|value| value.authority_watermark > record.authority_watermark)
        {
            let current = current
                .ok_or_else(|| Error::Infrastructure("serving current row disappeared".into()))?;
            tx.commit().await.map_err(db)?;
            return Ok(current);
        }
        let desired: i64 = sqlx::query_scalar("SELECT COALESCE(max(desired_authority_seq),0) FROM projection_watermarks WHERE subject_id=$1 AND family=$2 AND (space_signature=$3 OR space_signature='*')")
            .bind(record.subject.0).bind(&record.family).bind(&record.space).fetch_one(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO serving_generations(generation_id,subject_id,family,space_signature,authority_watermark,implementation_id,implementation_revision,config_digest,artifact_location,artifact_hash,state,built_at,published_at,metadata) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'ready',$11,$12,$13)")
            .bind(record.generation_id.0).bind(record.subject.0).bind(&record.family).bind(&record.space).bind(record.authority_watermark).bind(&record.implementation_id).bind(&record.implementation_revision).bind(&record.config_digest).bind(&record.artifact_location).bind(&record.artifact_hash).bind(record.built_at).bind(Utc::now()).bind(&record.metadata).execute(&mut *tx).await.map_err(db)?;
        if record.authority_watermark >= desired {
            sqlx::query("INSERT INTO serving_current(subject_id,family,space_signature,generation_id) VALUES($1,$2,$3,$4) ON CONFLICT(subject_id,family,space_signature) DO UPDATE SET generation_id=excluded.generation_id").bind(record.subject.0).bind(&record.family).bind(&record.space).bind(record.generation_id.0).execute(&mut *tx).await.map_err(db)?;
            if let Some(current) = current {
                sqlx::query(
                    "UPDATE serving_generations SET state='retired' WHERE generation_id=$1",
                )
                .bind(current.generation_id.0)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(record)
    }
}
