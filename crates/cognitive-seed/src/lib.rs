//! Canonical Cognitive Seed v1 parser and semantic-path producer.

use nous_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedDocument {
    pub schema_version: u32,
    #[serde(rename = "self")]
    pub self_section: Option<SelfSeed>,
    pub social: Option<SocialSeed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfSeed {
    #[serde(default)]
    pub facets: Vec<SelfFacetSeed>,
    #[serde(default)]
    pub narratives: Vec<SelfNarrativeSeed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfFacetSeed {
    pub kind: String,
    pub key: String,
    pub statement: String,
    #[serde(default = "default_scope")]
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfNarrativeSeed {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialSeed {
    #[serde(default)]
    pub relation_types: Vec<RelationTypeSeed>,
    #[serde(default)]
    pub relationships: Vec<RelationshipSeed>,
    #[serde(default)]
    pub conventions: Vec<LanguageConventionSeed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationTypeSeed {
    pub key: String,
    pub allowed_from: Vec<String>,
    pub allowed_to: Vec<String>,
    pub view: String,
    #[serde(default)]
    pub inverse_key: Option<String>,
    #[serde(default = "default_temporal")]
    pub temporal: String,
    #[serde(default)]
    pub degree: Option<DegreeSeed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DegreeSeed {
    pub kind: String,
    #[serde(default)]
    pub levels: Vec<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub states: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipSeed {
    pub key: String,
    #[serde(rename = "type")]
    pub relation_type: String,
    pub from: SeedParty,
    pub to: SeedParty,
    #[serde(default = "default_epistemic")]
    pub epistemic: String,
    #[serde(default)]
    pub valid_time: Option<SeedTime>,
    #[serde(default)]
    pub degree: Option<SeedDegreeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedDegreeValue {
    pub kind: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub number: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedParty {
    pub kind: String,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub r#ref: Option<String>,
}

impl SeedParty {
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref().or(self.r#ref.as_deref())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedTime {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageConventionSeed {
    pub key: String,
    pub expression: String,
    pub meaning: String,
    #[serde(default)]
    pub epistemic: String,
    #[serde(default)]
    pub pragmatic_role: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    pub scope: ConventionScopeSeed,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConventionScopeSeed {
    pub kind: String,
    #[serde(default)]
    pub parties: Vec<SeedParty>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub r#ref: Option<String>,
}

impl ConventionScopeSeed {
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref().or(self.r#ref.as_deref())
    }
}

pub fn parse(text: &str) -> Result<SeedDocument> {
    let document: SeedDocument = toml::from_str(text)
        .map_err(|error| Error::Invalid(format!("invalid Cognitive Seed v1 TOML: {error}")))?;
    document.validate()?;
    Ok(document)
}

impl SeedDocument {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(Error::Invalid(format!(
                "unsupported Cognitive Seed schema_version {}",
                self.schema_version
            )));
        }
        if let Some(self_seed) = &self.self_section {
            self_seed.validate()?;
        }
        if let Some(social) = &self.social {
            social.validate()?;
        }
        Ok(())
    }
}

impl SelfSeed {
    pub fn validate(&self) -> Result<()> {
        let allowed = [
            "identity",
            "role",
            "capability",
            "limitation",
            "tendency",
            "value",
            "preference",
        ];
        let mut keys = BTreeSet::new();
        for facet in &self.facets {
            if !allowed.contains(&facet.kind.as_str()) {
                return Err(Error::Invalid(format!(
                    "invalid Self facet kind: {}",
                    facet.kind
                )));
            }
            validate_key(&facet.key, "Self facet key")?;
            if !keys.insert((&facet.kind, &facet.key)) {
                return Err(Error::Conflict(format!(
                    "duplicate Self facet ({},{})",
                    facet.kind, facet.key
                )));
            }
            if facet.statement.trim().is_empty() || facet.statement.len() > 65_536 {
                return Err(Error::Invalid(
                    "Self facet statement bounds exceeded".into(),
                ));
            }
            validate_scope(&facet.scope)?;
        }
        let mut narrative_keys = BTreeSet::new();
        for narrative in &self.narratives {
            validate_key(&narrative.key, "Self narrative key")?;
            if !narrative_keys.insert(&narrative.key) {
                return Err(Error::Conflict(format!(
                    "duplicate Self narrative key: {}",
                    narrative.key
                )));
            }
            if narrative.text.trim().is_empty() || narrative.text.len() > 65_536 {
                return Err(Error::Invalid("Self narrative text bounds exceeded".into()));
            }
        }
        Ok(())
    }
}

impl SocialSeed {
    pub fn validate(&self) -> Result<()> {
        let mut type_keys = BTreeSet::new();
        for relation in &self.relation_types {
            validate_social_key(&relation.key, "relation type key")?;
            if !type_keys.insert(&relation.key) {
                return Err(Error::Conflict(format!(
                    "duplicate relation type key: {}",
                    relation.key
                )));
            }
            validate_party_kinds(&relation.allowed_from)?;
            validate_party_kinds(&relation.allowed_to)?;
            if relation.allowed_from.is_empty() || relation.allowed_to.is_empty() {
                return Err(Error::Invalid(
                    "relation type endpoints cannot be empty".into(),
                ));
            }
            if !matches!(relation.view.as_str(), "directed" | "symmetric" | "inverse") {
                return Err(Error::Invalid("invalid relation type view".into()));
            }
            if relation.view == "inverse" && relation.inverse_key.is_none() {
                return Err(Error::Invalid(
                    "inverse relation type needs inverse_key".into(),
                ));
            }
            if !matches!(relation.temporal.as_str(), "state" | "interval" | "instant") {
                return Err(Error::Invalid(
                    "invalid relation type temporal semantics".into(),
                ));
            }
            if let Some(degree) = &relation.degree {
                degree.validate()?;
            }
        }
        let mut relationship_keys = BTreeSet::new();
        for relationship in &self.relationships {
            validate_social_key(&relationship.key, "relationship seed key")?;
            if !relationship_keys.insert(&relationship.key) {
                return Err(Error::Conflict(format!(
                    "duplicate relationship seed key: {}",
                    relationship.key
                )));
            }
            validate_party(&relationship.from)?;
            validate_party(&relationship.to)?;
            if relationship.from.kind == "subject" && relationship.to.kind == "subject" {
                return Err(Error::Invalid(
                    "relationship endpoints must not be the same SubjectSelf".into(),
                ));
            }
        }
        let mut convention_keys = BTreeSet::new();
        for convention in &self.conventions {
            validate_social_key(&convention.key, "language convention key")?;
            if !convention_keys.insert(&convention.key) {
                return Err(Error::Conflict(format!(
                    "duplicate language convention key: {}",
                    convention.key
                )));
            }
            if convention.expression.trim().is_empty() || convention.expression.len() > 65_536 {
                return Err(Error::Invalid(
                    "language convention expression bounds exceeded".into(),
                ));
            }
            if convention.meaning.trim().is_empty() || convention.meaning.len() > 65_536 {
                return Err(Error::Invalid(
                    "language convention meaning bounds exceeded".into(),
                ));
            }
            validate_scope_kind(&convention.scope.kind)?;
            for party in &convention.scope.parties {
                validate_party(party)?;
            }
            if let Some(value) = convention.context.as_deref() {
                validate_scope_token(value)?;
            }
            if let Some(value) = convention.topic.as_deref() {
                validate_scope_token(value)?;
            }
        }
        Ok(())
    }
}

impl DegreeSeed {
    fn validate(&self) -> Result<()> {
        match self.kind.as_str() {
            "ordinal" if (2..=16).contains(&self.levels.len()) => Ok(()),
            "bounded_scalar"
                if self.min.is_some_and(f64::is_finite)
                    && self.max.is_some_and(f64::is_finite)
                    && self.min < self.max =>
            {
                Ok(())
            }
            "typed_state" if (1..=32).contains(&self.states.len()) => Ok(()),
            "none"
                if self.levels.is_empty()
                    && self.states.is_empty()
                    && self.min.is_none()
                    && self.max.is_none() =>
            {
                Ok(())
            }
            _ => Err(Error::Invalid("invalid relation degree semantics".into())),
        }
    }
}

fn validate_party(party: &SeedParty) -> Result<()> {
    if !matches!(
        party.kind.as_str(),
        "subject" | "person" | "group" | "community" | "channel"
    ) {
        return Err(Error::Invalid("invalid social party kind".into()));
    }
    if party.kind == "subject" {
        if party.reference().is_some() {
            return Err(Error::Invalid(
                "Subject party cannot carry a reference".into(),
            ));
        }
    } else if party.reference().is_none() {
        return Err(Error::Invalid(
            "external social party needs a reference".into(),
        ));
    }
    Ok(())
}

fn validate_party_kinds(values: &[String]) -> Result<()> {
    values.iter().try_for_each(|value| {
        if matches!(
            value.as_str(),
            "subject" | "person" | "group" | "community" | "channel"
        ) {
            Ok(())
        } else {
            Err(Error::Invalid("invalid allowed social party kind".into()))
        }
    })
}

fn validate_scope_kind(value: &str) -> Result<()> {
    if matches!(value, "person" | "dyad" | "group" | "community" | "channel") {
        Ok(())
    } else {
        Err(Error::Invalid("invalid SocialScope kind".into()))
    }
}

fn validate_scope_token(value: &str) -> Result<()> {
    if (1..=128).contains(&value.len())
        && value.bytes().enumerate().all(|(index, byte)| {
            (index == 0 && byte.is_ascii_lowercase())
                || (index > 0
                    && (byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || b"._:-".contains(&byte)))
        })
    {
        Ok(())
    } else {
        Err(Error::Invalid("invalid social scope token".into()))
    }
}

fn validate_key(value: &str, label: &str) -> Result<()> {
    if (1..=128).contains(&value.len())
        && value.bytes().enumerate().all(|(index, byte)| {
            (index == 0 && byte.is_ascii_lowercase())
                || (index > 0
                    && (byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || b"._-".contains(&byte)))
        })
    {
        Ok(())
    } else {
        Err(Error::Invalid(format!("invalid {label}")))
    }
}

fn validate_social_key(value: &str, label: &str) -> Result<()> {
    if (1..=128).contains(&value.len())
        && value.bytes().enumerate().all(|(index, byte)| {
            (index == 0 && byte.is_ascii_lowercase())
                || (index > 0
                    && (byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || b"._:-".contains(&byte)))
        })
    {
        Ok(())
    } else {
        Err(Error::Invalid(format!("invalid {label}")))
    }
}

fn validate_scope(value: &str) -> Result<()> {
    if value == "global" {
        return Ok(());
    }
    if value.len() > 256 || value.chars().any(char::is_whitespace) {
        return Err(Error::Invalid("invalid Self scope".into()));
    }
    let (namespace, value) = value
        .split_once(':')
        .ok_or_else(|| Error::Invalid("Self scope must be global or namespace:value".into()))?;
    if namespace.is_empty() || value.is_empty() {
        return Err(Error::Invalid(
            "Self scope namespace and value are required".into(),
        ));
    }
    Ok(())
}

fn default_scope() -> String {
    "global".into()
}

fn default_temporal() -> String {
    "state".into()
}

fn default_epistemic() -> String {
    "reported".into()
}

pub fn self_facet_path(value: &SelfFacetSeed) -> String {
    format!("self.facets/{}/{}", value.kind, value.key)
}

pub fn self_narrative_path(value: &SelfNarrativeSeed) -> String {
    format!("self.narratives/{}", value.key)
}

pub fn relation_type_path(value: &RelationTypeSeed) -> String {
    format!("social.relation-types/{}", value.key)
}

pub fn relationship_path(value: &RelationshipSeed) -> String {
    format!("social.relationships/{}", value.key)
}

pub fn convention_path(value: &LanguageConventionSeed) -> String {
    format!("social.conventions/{}", value.key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiline_narrative_and_stable_paths() {
        let document = parse(
            r#"
schema_version = 1

[[self.facets]]
kind = "identity"
key = "primary-name"
statement = "Nous"

[[self.narratives]]
key = "primary"
text = """
A persistent subject.
It has a continuous history.
"""
"#,
        )
        .unwrap();
        let narrative = &document.self_section.unwrap().narratives[0];
        assert!(narrative.text.contains("continuous history"));
        assert_eq!(self_narrative_path(narrative), "self.narratives/primary");
    }

    #[test]
    fn unknown_fields_and_duplicate_keys_are_rejected() {
        assert!(parse("schema_version = 1\nunknown = true\n").is_err());
        assert!(parse(
            "schema_version=1\n[[self.facets]]\nkind='identity'\nkey='a'\nstatement='x'\n[[self.facets]]\nkind='identity'\nkey='a'\nstatement='y'\n"
        )
        .is_err());
    }
}
