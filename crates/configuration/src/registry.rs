use crate::key::*;
use nous_core::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

pub type ConfigValidator = Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct ConfigDescriptor {
    pub path: String,
    pub owner: String,
    pub human_description: String,
    pub value_type_name: String,
    pub reference_default_json: Value,
    pub exposure: ConfigExposure,
    pub scope_policy: ConfigScopePolicy,
    pub apply_mode: ConfigApplyMode,
    pub semantic_effect: ConfigSemanticEffect,
    validator: ConfigValidator,
}

impl ConfigDescriptor {
    pub fn validate(&self, value: &Value) -> Result<()> {
        (self.validator)(value)
    }
}

#[derive(Clone)]
pub struct ConfigRegistry {
    descriptors: Arc<BTreeMap<String, ConfigDescriptor>>,
    digest: String,
}

impl ConfigRegistry {
    pub fn descriptor(&self, path: &str) -> Option<&ConfigDescriptor> {
        self.descriptors.get(path)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &ConfigDescriptor> {
        self.descriptors.values()
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.descriptors.keys().map(String::as_str)
    }
}

pub struct ConfigRegistryBuilder {
    descriptors: BTreeMap<String, ConfigDescriptor>,
}

impl Default for ConfigRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRegistryBuilder {
    pub fn new() -> Self {
        Self {
            descriptors: BTreeMap::new(),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "registration keeps the complete descriptor contract at the owner boundary"
    )]
    pub fn register<T>(
        &mut self,
        key: ConfigKey<T>,
        owner: impl Into<String>,
        human_description: impl Into<String>,
        default: T,
        exposure: ConfigExposure,
        scope_policy: ConfigScopePolicy,
        apply_mode: ConfigApplyMode,
        semantic_effect: ConfigSemanticEffect,
        validator: impl Fn(&T) -> Result<()> + Send + Sync + 'static,
    ) -> Result<()>
    where
        T: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let path = key.path();
        if !valid_key_path(path) {
            return Err(Error::Invalid(format!("invalid configuration key: {path}")));
        }
        if self.descriptors.contains_key(path) {
            return Err(Error::Conflict(format!(
                "configuration key is registered twice: {path}"
            )));
        }
        let default_json = serde_json::to_value(&default)
            .map_err(|error| Error::Invalid(format!("default value for {path}: {error}")))?;
        validator(&default)?;
        let type_name = std::any::type_name::<T>().to_owned();
        let validator = Arc::new(move |value: &Value| {
            let parsed: T = serde_json::from_value(value.clone())
                .map_err(|error| Error::Invalid(format!("invalid value for {path}: {error}")))?;
            validator(&parsed)
        });
        self.descriptors.insert(
            path.to_owned(),
            ConfigDescriptor {
                path: path.to_owned(),
                owner: owner.into(),
                human_description: human_description.into(),
                value_type_name: type_name,
                reference_default_json: default_json,
                exposure,
                scope_policy,
                apply_mode,
                semantic_effect,
                validator,
            },
        );
        Ok(())
    }

    pub fn finish(self) -> Result<ConfigRegistry> {
        let digest_input = self
            .descriptors
            .values()
            .map(|descriptor| {
                serde_json::json!({
                    "key": descriptor.path,
                    "value_type_name": descriptor.value_type_name,
                    "reference_default_json": descriptor.reference_default_json,
                    "owner": descriptor.owner,
                    "exposure": descriptor.exposure,
                    "scope_policy": descriptor.scope_policy,
                    "apply_mode": descriptor.apply_mode,
                    "semantic_effect": descriptor.semantic_effect,
                })
            })
            .collect::<Vec<_>>();
        let digest = blake3::hash(
            serde_json::to_string(&digest_input)
                .map_err(|error| Error::Internal(error.to_string()))?
                .as_bytes(),
        )
        .to_hex()
        .to_string();
        Ok(ConfigRegistry {
            descriptors: Arc::new(self.descriptors),
            digest,
        })
    }
}

pub fn flatten_settings(value: &Value) -> Result<BTreeMap<String, Value>> {
    let settings = value
        .get("settings")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let Value::Object(settings) = settings else {
        return Err(Error::Invalid("[settings] must be a table".into()));
    };
    let mut output = BTreeMap::new();
    flatten_object("", &settings, &mut output)?;
    Ok(output)
}

fn flatten_object(
    prefix: &str,
    object: &serde_json::Map<String, Value>,
    output: &mut BTreeMap<String, Value>,
) -> Result<()> {
    for (name, value) in object {
        if name.is_empty() || name.contains('.') {
            return Err(Error::Invalid(format!("invalid settings key: {name}")));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            Value::Object(child) => flatten_object(&path, child, output)?,
            _ => {
                output.insert(path, value.clone());
            }
        }
    }
    Ok(())
}
