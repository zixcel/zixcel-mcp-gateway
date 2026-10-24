//! Closed configuration model; no path, endpoint, command, or credential fields exist.

use crate::document::PublicDocument;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<PublicDocument>,
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
        if u64::try_from(input.len()).map_err(|_| "config size overflow")? > MAX_CONFIG_BYTES {
            return Err("config exceeds 64 KiB".into());
        }
        let parsed: toml::Value =
            toml::from_str(input).map_err(|_| "config does not match the closed schema")?;
        let version = parsed
            .get("schema_version")
            .and_then(toml::Value::as_integer);
        // Preserve the v1 metadata marker policy. Approved v2 public prose can
        // discuss credentials/security; classification is an operator decision.
        if version != Some(2) && has_sensitive_marker(input) {
            return Err("config contains a prohibited secret-like marker".into());
        }
        let config: Self = parsed
            .try_into()
            .map_err(|_| "config does not match the closed schema")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if !matches!(self.schema_version, 1 | 2) {
            return Err("unsupported schema_version".into());
        }
        validate_id(self.server.id.as_str())?;
        validate_text(&self.server.title, 80)?;
        validate_text(&self.server.instructions, 500)?;
        if self.schema_version == 2
            && (has_sensitive_marker(&self.server.id.0)
                || has_sensitive_marker(&self.server.title)
                || has_sensitive_marker(&self.server.instructions))
        {
            return Err("metadata contains a prohibited marker".into());
        }
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
        let mut document_bytes = 0usize;
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
            if self.schema_version == 2 {
                let metadata =
                    serde_json::to_string(resource).map_err(|_| "invalid public metadata")?;
                if has_sensitive_marker(&metadata) {
                    return Err("metadata contains a prohibited marker".into());
                }
            }
            if let Some(document) = &resource.document {
                if self.schema_version != 2 {
                    return Err("documents require schema_version 2".into());
                }
                document.validate()?;
                document_bytes = document_bytes
                    .checked_add(document.text.len())
                    .ok_or("document byte overflow")?;
                if document_bytes > 32768 {
                    return Err("aggregate documents exceed 32 KiB".into());
                }
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
