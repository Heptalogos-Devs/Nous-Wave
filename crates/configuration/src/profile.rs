//! Versioned reference defaults for a jointly tuned algorithm family.
use crate::{ConfigKey, ConfigRegistryBuilder};
use nous_core::{Error, Result};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceProfile {
    pub identity: String,
    pub revision: u32,
    pub values: BTreeMap<String, Value>,
}
impl ReferenceProfile {
    pub fn parse(source: &str) -> Result<Self> {
        let profile: Self = serde_json::from_str(source)
            .map_err(|error| Error::Internal(format!("reference profile: {error}")))?;
        if profile.identity.is_empty() || profile.revision == 0 || profile.values.is_empty() {
            return Err(Error::Internal("invalid reference profile identity".into()));
        }
        Ok(profile)
    }
    pub fn get<T: DeserializeOwned>(&self, key: ConfigKey<T>) -> Result<T> {
        serde_json::from_value(
            self.values.get(key.path()).cloned().ok_or_else(|| {
                Error::Internal(format!("reference default missing: {}", key.path()))
            })?,
        )
        .map_err(|error| Error::Internal(format!("reference default {}: {error}", key.path())))
    }
    pub fn describe(&self, registry: &mut ConfigRegistryBuilder) -> Result<()> {
        for key in self.values.keys() {
            registry.describe(key, |d| {
                d.reference_profile = Some(format!("{}@{}", self.identity, self.revision));
            })?;
        }
        Ok(())
    }
}
