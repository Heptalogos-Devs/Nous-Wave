use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeedImportResult {
    pub created: Vec<String>,
    pub unchanged: Vec<String>,
    pub conflicts: Vec<String>,
}

impl SelfService {
    pub async fn import_seed(
        &self,
        subject: SubjectId,
        seed_version_id: nous_core::CognitiveSeedVersionId,
        text: &str,
    ) -> Result<SeedImportResult> {
        let document = nous_cognitive_seed::parse(text)?;
        let mut result = SeedImportResult::default();
        let Some(seed) = document.self_section else {
            return Ok(result);
        };
        for facet in seed.facets {
            let path = format!("self.facets/{}/{}", facet.kind, facet.key);
            let operation_id = seed_operation(subject, seed_version_id, &path);
            let input = CreateSelfFacet {
                operation_id,
                subject,
                kind: SelfFacetKind::try_from(facet.kind.as_str())?,
                key: facet.key,
                statement: facet.statement,
                scope: facet.scope,
                epistemic_class: EpistemicClass::Reported,
                valid_time: TemporalExtent::Unknown,
                formed_at: Utc::now(),
                supports: vec![RevisionSupport::Seed(nous_core::SeedSupportRef::new(
                    seed_version_id,
                    path.clone(),
                )?)],
                producer_signature_id: None,
            };
            match self.create_facet(input).await {
                Ok(_) => result.created.push(path),
                Err(nous_core::Error::Conflict(_)) => result.conflicts.push(path),
                Err(error) => return Err(error),
            }
        }
        for narrative in seed.narratives {
            let path = format!("self.narratives/{}", narrative.key);
            let operation_id = seed_operation(subject, seed_version_id, &path);
            let input = CreateNarrativeIdentity {
                operation_id,
                subject,
                key: narrative.key,
                text: narrative.text,
                valid_time: TemporalExtent::Unknown,
                formed_at: Utc::now(),
                supports: vec![RevisionSupport::Seed(nous_core::SeedSupportRef::new(
                    seed_version_id,
                    path.clone(),
                )?)],
                references: Vec::new(),
                producer_signature_id: None,
            };
            match self.create_narrative(input).await {
                Ok(_) => result.created.push(path),
                Err(nous_core::Error::Conflict(_)) => result.conflicts.push(path),
                Err(error) => return Err(error),
            }
        }
        Ok(result)
    }
}

fn seed_operation(
    subject: SubjectId,
    seed_version_id: nous_core::CognitiveSeedVersionId,
    path: &str,
) -> OperationId {
    let name = format!("{}\n{}\n{}", subject.0, seed_version_id.0, path);
    OperationId(Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes()))
}
