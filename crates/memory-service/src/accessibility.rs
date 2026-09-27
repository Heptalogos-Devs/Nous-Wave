use crate::*;
use nous_authority_store::database_error as db;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityPolicy {
    pub epsilon: f64,
    pub tau_days: f64,
    pub decay: f64,
    pub formation_weight: f64,
}

impl Default for AccessibilityPolicy {
    fn default() -> Self {
        Self {
            epsilon: 0.02,
            tau_days: 30.0,
            decay: 0.5,
            formation_weight: 1.0,
        }
    }
}

impl AccessibilityPolicy {
    pub fn validate(self) -> Result<Self> {
        if self.epsilon <= 0.0
            || self.tau_days <= 0.0
            || self.decay <= 0.0
            || self.formation_weight < 0.0
        {
            return Err(Error::Invalid(
                "invalid accessibility reference parameters".into(),
            ));
        }
        Ok(self)
    }

    pub fn activation(&self, age_days: f64, uses: &[(String, f64)]) -> f64 {
        let contribution = |weight: f64, delta: f64| {
            weight * (1.0 + delta.max(0.0) / self.tau_days).powf(-self.decay)
        };
        let mut mass = self.epsilon + contribution(self.formation_weight, age_days);
        for (kind, delta) in uses {
            mass += contribution(use_weight(kind), *delta);
        }
        mass.ln()
    }

    pub fn level_from_activation(&self, activation: f64) -> AccessibilityLevel {
        if activation >= -0.80 {
            AccessibilityLevel::Normal
        } else if activation >= -1.60 {
            AccessibilityLevel::Deep
        } else {
            AccessibilityLevel::Explicit
        }
    }

    pub async fn level_for_memory(
        &self,
        store: &AuthorityStore,
        subject: SubjectId,
        memory: MemoryId,
        now: DateTime<Utc>,
    ) -> Result<AccessibilityLevel> {
        let row = sqlx::query("SELECT accessibility_mode,created_at FROM memory_objects WHERE subject_id=$1 AND memory_id=$2")
            .bind(subject.0)
            .bind(memory.0)
            .fetch_optional(store.pool())
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("memory not found".into()))?;
        let mode: AccessibilityMode = serde_json::from_value(serde_json::Value::String(
            row.try_get::<String, _>("accessibility_mode").map_err(db)?,
        ))
        .map_err(|_| Error::Infrastructure("invalid accessibility mode".into()))?;
        let created: DateTime<Utc> = row.try_get("created_at").map_err(db)?;
        if !matches!(mode, AccessibilityMode::Auto) {
            return Ok(match mode {
                AccessibilityMode::Normal => AccessibilityLevel::Normal,
                AccessibilityMode::Deep => AccessibilityLevel::Deep,
                AccessibilityMode::Explicit => AccessibilityLevel::Explicit,
                AccessibilityMode::Auto => unreachable!(),
            });
        }
        let uses = sqlx::query("SELECT use_kind,occurred_at FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind='memory_revision' AND ref_value IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$2) AND use_kind <> 'presented'")
            .bind(subject.0)
            .bind(memory.0)
            .fetch_all(store.pool())
            .await
            .map_err(db)?
            .into_iter()
            .map(|row| {
                let kind: String = row.try_get("use_kind").map_err(db)?;
                let at: DateTime<Utc> = row.try_get("occurred_at").map_err(db)?;
                Ok((kind, (now - at).num_seconds().max(0) as f64 / 86400.0))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(self.level_from_activation(
            self.activation((now - created).num_seconds().max(0) as f64 / 86400.0, &uses),
        ))
    }
}

fn use_weight(kind: &str) -> f64 {
    match kind {
        "referenced" => 1.0,
        "acted_on" => 1.4,
        "result_supported" => 1.1,
        "result_refuted" => 1.2,
        "corrected" => 1.5,
        "pinned" => 2.0,
        "presented" => 0.0,
        _ => 0.0,
    }
}

pub fn eligible(level: AccessibilityLevel, effort: CognitiveEffort, exact: bool) -> bool {
    if exact {
        return true;
    }
    match level {
        AccessibilityLevel::Normal => true,
        AccessibilityLevel::Deep => {
            matches!(effort, CognitiveEffort::Deep | CognitiveEffort::Maximum)
        }
        AccessibilityLevel::Explicit => false,
    }
}

impl MemoryService {
    pub async fn set_accessibility(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
        mode: AccessibilityMode,
    ) -> Result<MemoryView> {
        let digest = operation_digest(
            "set_accessibility",
            subject,
            &serde_json::json!({"memory_id":memory,"expected_object_epoch":expected_object_epoch,"mode":mode}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) =
            check_receipt(&mut tx, subject, operation_id, "set_accessibility", &digest).await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self.memory(subject, memory, None).await;
            }
            return Err(Error::Unavailable(
                "accessibility operation is already in progress".into(),
            ));
        }
        let epoch:i64=sqlx::query_scalar("SELECT object_epoch FROM memory_objects WHERE subject_id=$1 AND memory_id=$2 FOR UPDATE").bind(subject.0).bind(memory.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||Error::NotFound("memory not found".into()))?;
        if epoch != expected_object_epoch {
            return Err(Error::Conflict("expected object epoch is stale".into()));
        }
        sqlx::query("UPDATE memory_objects SET accessibility_mode=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND memory_id=$2").bind(subject.0).bind(memory.0).bind(format!("{mode:?}").to_lowercase()).execute(&mut *tx).await.map_err(db)?;
        AuthorityStore::invalidate_in(
            &mut tx,
            subject,
            ProjectionInvalidation {
                exact: true,
                ..Default::default()
            },
        )
        .await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "memory",
            Some(&memory.0.to_string()),
            None,
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.memory(subject, memory, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_values_match_the_frozen_profile() {
        let policy = AccessibilityPolicy::default();
        assert!((policy.activation(0.0, &[]) - 0.019802627).abs() < 1e-6);
        assert_eq!(
            policy.level_from_activation(policy.activation(180.0, &[])),
            AccessibilityLevel::Deep
        );
        assert_eq!(
            policy.level_from_activation(policy.activation(1000.0, &[])),
            AccessibilityLevel::Explicit
        );
    }

    #[test]
    fn meaningful_use_weights_and_mode_gates_are_distinct() {
        let policy = AccessibilityPolicy::default();
        let base = policy.activation(0.0, &[]);
        for kind in [
            "referenced",
            "acted_on",
            "result_supported",
            "result_refuted",
            "corrected",
            "pinned",
        ] {
            assert!(policy.activation(0.0, &[(kind.into(), 0.0)]) > base);
        }
        assert_eq!(policy.activation(0.0, &[("presented".into(), 0.0)]), base);
        assert!(eligible(
            AccessibilityLevel::Deep,
            CognitiveEffort::Deep,
            false
        ));
        assert!(!eligible(
            AccessibilityLevel::Deep,
            CognitiveEffort::Normal,
            false
        ));
        assert!(!eligible(
            AccessibilityLevel::Explicit,
            CognitiveEffort::Maximum,
            false
        ));
        assert!(eligible(
            AccessibilityLevel::Explicit,
            CognitiveEffort::Light,
            true
        ));
    }
}
