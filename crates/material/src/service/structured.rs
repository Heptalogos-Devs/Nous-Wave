use super::*;
use nous_persistence::database_error as db;
use std::collections::HashSet;

fn requested_supports(payload: &serde_json::Value) -> Result<HashSet<CognitiveRef>> {
    let mut requested = HashSet::new();
    let summary = payload
        .get("summary")
        .filter(|value| value.is_object())
        .ok_or_else(|| Error::Invalid("structured summary missing".into()))?;
    let mut fields = vec![summary];
    for group in [
        "observations",
        "mentions",
        "embedded_text",
        "source_text",
        "speech",
        "interpretations",
        "uncertainties",
    ] {
        let items = payload
            .get(group)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                Error::Invalid("structured interpretation field group missing".into())
            })?;
        if items.len() > 64 {
            return Err(Error::Invalid(
                "structured interpretation field count exceeded".into(),
            ));
        }
        fields.extend(items);
    }
    for item in fields {
        if item.get("support_keys").is_some() {
            return Err(Error::Invalid(
                "invocation-local support keys cannot be persisted".into(),
            ));
        }
        let supports = item
            .get("supports")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| Error::Invalid("structured field lacks stable supports".into()))?;
        if supports.len() > 16
            || ((item.get("basis").and_then(serde_json::Value::as_str) == Some("direct")
                || (std::ptr::eq(item, summary)
                    && summary
                        .get("content")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|text| !text.trim().is_empty())))
                && supports.is_empty())
        {
            return Err(Error::Invalid(
                "structured field support bounds invalid".into(),
            ));
        }
        for support in supports {
            let kind = support
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::Invalid("invalid support kind".into()))?;
            let value = support
                .get("value")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::Invalid("invalid support identity".into()))?;
            requested.insert(nous_core::parse_reference(kind, value)?);
        }
    }
    Ok(requested)
}

impl MaterialService {
    pub(super) async fn validate_field_supports(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        representation: &DerivedRepresentation,
    ) -> Result<()> {
        let Some(payload) = &representation.payload_json else {
            return Ok(());
        };
        let requested = requested_supports(payload)?;
        let mut allowed: HashSet<_> = representation
            .inputs
            .iter()
            .map(|item| item.reference.clone())
            .collect();
        let representations: Vec<_> = representation
            .inputs
            .iter()
            .filter_map(|item| match item.reference {
                CognitiveRef::DerivedRepresentation(id) => Some(id.0),
                _ => None,
            })
            .collect();
        let regions: Vec<_> = representation
            .inputs
            .iter()
            .filter_map(|item| match item.reference {
                CognitiveRef::DerivedRegion(id) => Some(id.0),
                _ => None,
            })
            .collect();
        let rows = sqlx::query("WITH RECURSIVE ancestry(id) AS (SELECT derived_representation_id FROM derived_representations WHERE subject_id=$1 AND derived_representation_id=ANY($2) UNION SELECT derived_representation_id FROM derived_regions WHERE subject_id=$1 AND derived_region_id=ANY($3) UNION SELECT COALESCE(i.input_representation_id,r.derived_representation_id) FROM ancestry a JOIN derived_representation_inputs i ON i.derived_representation_id=a.id LEFT JOIN derived_regions r ON r.derived_region_id=i.derived_region_id WHERE i.input_representation_id IS NOT NULL OR r.derived_representation_id IS NOT NULL) SELECT 'derived_representation' AS kind,id FROM ancestry UNION SELECT 'derived_region',r.derived_region_id FROM derived_regions r JOIN ancestry a ON a.id=r.derived_representation_id WHERE r.subject_id=$1 UNION SELECT 'source_region',i.source_region_id FROM derived_representation_inputs i JOIN ancestry a ON a.id=i.derived_representation_id WHERE i.source_region_id IS NOT NULL")
            .bind(representation.subject_id.0).bind(representations).bind(regions).fetch_all(&mut **tx).await.map_err(db)?;
        for row in rows {
            let kind: String = row.try_get("kind").map_err(db)?;
            let id: Uuid = row.try_get("id").map_err(db)?;
            allowed.insert(nous_core::parse_reference(&kind, &id.to_string())?);
        }
        if !requested.is_subset(&allowed) {
            return Err(Error::Invalid(
                "structured field support is outside its input graph".into(),
            ));
        }
        Ok(())
    }
}
