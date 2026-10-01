//! Closed configuration model; no path, endpoint, command, or credential fields exist.

use crate::file_input::read_bounded_regular_file;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_RESOURCES: usize = 64;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PublicId(String);

impl PublicId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    pub id: PublicId,
    pub title: String,
    pub instructions: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicResource {
    pub id: PublicId,
    pub title: String,
    pub summary: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u8,
    pub server: Server,
    #[serde(rename = "resource")]
    pub resources: Vec<PublicResource>,
}

impl Config {
    /// Loads a validated catalog from one explicit local file.
    ///
    /// # Errors
    ///
    /// Returns an error for inaccessible, linked, oversized, or invalid input.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = read_bounded_regular_file(path, MAX_CONFIG_BYTES)?;
        let input =
            String::from_utf8(bytes).map_err(|_| "config must be valid UTF-8".to_string())?;
        Self::from_toml(&input)
    }

    /// Parses and validates the closed public catalog schema.
    ///
    /// # Errors
    ///
    /// Returns an error for schema violations or secret-like markers.
    pub fn from_toml(input: &str) -> Result<Self, String> {
        if has_sensitive_marker(input) {
            return Err("config contains a prohibited secret-like marker".into());
        }
        let config: Self =
            toml::from_str(input).map_err(|_| "config does not match the closed schema")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("unsupported schema_version".into());
        }
        validate_id(self.server.id.as_str())?;
        validate_text(&self.server.title, 80)?;
        validate_text(&self.server.instructions, 500)?;
        let ids = self
            .resources
            .iter()
            .map(|resource| resource.id.as_str())
            .collect::<BTreeSet<_>>();
        if self.resources.is_empty()
            || self.resources.len() > MAX_RESOURCES
            || ids.len() != self.resources.len()
        {
            return Err("config must define 1 to 64 unique public resources".into());
        }
        for resource in &self.resources {
            validate_id(resource.id.as_str())?;
            validate_text(&resource.title, 80)?;
            validate_text(&resource.summary, 500)?;
            if resource.capabilities.is_empty() || resource.capabilities.len() > 16 {
                return Err("capabilities must contain 1 to 16 entries".into());
            }
            for capability in &resource.capabilities {
                validate_text(capability, 120)?;
            }
        }
        Ok(())
    }
}

fn validate_id(value: &str) -> Result<(), String> {
    let valid = (1..=63).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err("public IDs must be lowercase kebab-case identifiers".into())
    }
}

fn validate_text(value: &str, max: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err("public text is empty, oversized, or contains control characters".into());
    }
    Ok(())
}

fn has_sensitive_marker(input: &str) -> bool {
    let lowered = input.to_ascii_lowercase();
    [
        "password",
        "secret",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "authorization",
        "bearer ",
        "private_key",
        "-----begin",
        "connection_string",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}
