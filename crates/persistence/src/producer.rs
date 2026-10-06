// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use crate::{AuthorityStore, database_error};
use nous_core::{Error, ProducerSignature, Result};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

impl AuthorityStore {
    pub fn canonical_producer(producer: &ProducerSignature) -> Result<ProducerSignature> {
        for field in [
            &producer.provider_class,
            &producer.implementation,
            &producer.preprocessing_identity,
            &producer.preprocessing_revision,
            &producer.config_digest,
        ] {
            if field.trim().is_empty() || field.len() > 4096 {
                return Err(Error::Invalid("invalid producer signature fields".into()));
            }
        }
        if producer
            .model_identity
            .as_ref()
            .is_some_and(|v| v.trim().is_empty() || v.len() > 512)
            || producer
                .model_revision
                .as_ref()
                .is_some_and(|v| v.trim().is_empty() || v.len() > 512)
        {
            return Err(Error::Invalid("invalid producer model identity".into()));
        }
        let mut canonical = producer.clone();
        if producer.output_schema_digest.as_ref().is_some_and(|value| {
            value.len() != 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            return Err(Error::Invalid("invalid output schema digest".into()));
        }
        canonical.signature_hash.clear();
        canonical.signature_hash = blake3::hash(
            &serde_json::to_vec(&canonical).map_err(|e| Error::Invalid(e.to_string()))?,
        )
        .to_hex()
        .to_string();
        Ok(canonical)
    }
    pub async fn register_producer_in(
        tx: &mut Transaction<'_, Postgres>,
        producer: &ProducerSignature,
    ) -> Result<Uuid> {
        let p = Self::canonical_producer(producer)?;
        sqlx::query_scalar("INSERT INTO producer_signatures(producer_signature_id,signature_hash,provider_class,operation,implementation,model_identity,model_revision,preprocessing_identity,preprocessing_revision,config_digest,output_schema_digest,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,now()) ON CONFLICT(signature_hash) DO UPDATE SET signature_hash=excluded.signature_hash RETURNING producer_signature_id")
            .bind(Uuid::now_v7()).bind(p.signature_hash).bind(p.provider_class).bind(p.operation.as_str()).bind(p.implementation).bind(p.model_identity).bind(p.model_revision).bind(p.preprocessing_identity).bind(p.preprocessing_revision).bind(p.config_digest).bind(p.output_schema_digest).fetch_one(&mut **tx).await.map_err(database_error)
    }
}
