use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, str::FromStr};

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigExposure {
    Standard,
    Advanced,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigScopePolicy {
    SystemOnly,
    SubjectOverrideAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigApplyMode {
    Live,
    ServingRebuild,
    NewSubjectsOnly,
    RestartProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSemanticEffect {
    Operational,
    QueryPolicy,
    ServingProjection,
    AuthorityFormation,
    SubjectProvisioning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSource {
    ReferenceDefault,
    DeploymentFile,
    PersistedSystem,
    PersistedSubject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigActorTier {
    StandardUser,
    AdvancedUser,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessCapabilities {
    pub memory: bool,
    pub self_cognition: bool,
    pub social: bool,
}

impl Default for ProcessCapabilities {
    fn default() -> Self {
        Self {
            memory: true,
            self_cognition: false,
            social: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectCapabilities {
    pub memory: bool,
    pub self_cognition: bool,
    pub social: bool,
}

impl Default for SubjectCapabilities {
    fn default() -> Self {
        Self {
            memory: true,
            self_cognition: false,
            social: false,
        }
    }
}

impl SubjectCapabilities {
    pub fn require_process(self, process: ProcessCapabilities) -> Result<Self> {
        if self.memory && !process.memory {
            return Err(Error::Invalid(
                "subject memory capability is unavailable in this process".into(),
            ));
        }
        if self.self_cognition && !process.self_cognition {
            return Err(Error::Invalid(
                "subject Self capability is unavailable in this process".into(),
            ));
        }
        if self.social && !process.social {
            return Err(Error::Invalid(
                "subject Social capability is unavailable in this process".into(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigKey<T: 'static> {
    path: &'static str,
    marker: PhantomData<fn() -> T>,
}

impl<T: 'static> ConfigKey<T> {
    pub const fn new(path: &'static str) -> Self {
        Self {
            path,
            marker: PhantomData,
        }
    }

    pub const fn path(self) -> &'static str {
        self.path
    }
}

impl<T: 'static> From<ConfigKey<T>> for ConfigPath {
    fn from(value: ConfigKey<T>) -> Self {
        Self(value.path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConfigPath(pub &'static str);

impl FromStr for ConfigPath {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if !valid_key_path(value) {
            return Err(Error::Invalid(format!(
                "invalid configuration key: {value}"
            )));
        }
        Ok(Self(Box::leak(value.to_owned().into_boxed_str())))
    }
}

pub fn valid_key_path(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_lowercase()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

pub const PROCESS_MEMORY: ConfigKey<bool> = ConfigKey::new("capabilities.process.memory");
pub const PROCESS_SELF: ConfigKey<bool> = ConfigKey::new("capabilities.process.self_cognition");
pub const PROCESS_SOCIAL: ConfigKey<bool> = ConfigKey::new("capabilities.process.social");
pub const SUBJECT_DEFAULT_MEMORY: ConfigKey<bool> =
    ConfigKey::new("capabilities.subject_defaults.memory");
pub const SUBJECT_DEFAULT_SELF: ConfigKey<bool> =
    ConfigKey::new("capabilities.subject_defaults.self_cognition");
pub const SUBJECT_DEFAULT_SOCIAL: ConfigKey<bool> =
    ConfigKey::new("capabilities.subject_defaults.social");

pub fn process_capabilities(snapshot: &crate::ConfigSnapshot) -> Result<ProcessCapabilities> {
    Ok(ProcessCapabilities {
        memory: snapshot.get(PROCESS_MEMORY)?,
        self_cognition: snapshot.get(PROCESS_SELF)?,
        social: snapshot.get(PROCESS_SOCIAL)?,
    })
}

pub fn subject_default_capabilities(
    snapshot: &crate::ConfigSnapshot,
) -> Result<SubjectCapabilities> {
    Ok(SubjectCapabilities {
        memory: snapshot.get(SUBJECT_DEFAULT_MEMORY)?,
        self_cognition: snapshot.get(SUBJECT_DEFAULT_SELF)?,
        social: snapshot.get(SUBJECT_DEFAULT_SOCIAL)?,
    })
}
