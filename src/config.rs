//! Closed configuration model; no path, endpoint, command, or credential fields exist.

use crate::config_lexical::has_sensitive_marker;
use crate::document::PublicDocument;
use crate::file_input::read_bounded_regular_file;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;

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
        crate::config_validation::validate(&config)?;
        Ok(config)
    }
}
