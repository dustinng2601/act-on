//! Event payload handling.

use std::collections::HashMap;

use serde_yaml::Value;

/// Decoded event payload (`github.event.*`).
pub struct Event {
    pub raw: serde_json::Value,
}

impl Event {
    /// Parse a JSON event payload file (`GITHUB_EVENT_PATH`).
    pub fn from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        let v: serde_json::Value = serde_json::from_slice(&bytes)?;
        Ok(Self { raw: v })
    }

    /// Build an empty synthetic event (used when `--eventpath` not given).
    pub fn empty() -> Self {
        Self {
            raw: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Build a synthetic `workflow_dispatch` event with `inputs` populated.
    pub fn workflow_dispatch(inputs: &HashMap<String, String>) -> Self {
        let mut obj = serde_json::Map::new();
        obj.insert("inputs".into(), serde_json::json!(inputs));
        Self {
            raw: serde_json::Value::Object(obj),
        }
    }

    /// Convert into a `serde_yaml::Value` for storage inside `GithubContext.event`.
    pub fn to_yaml(&self) -> Value {
        let s = serde_json::to_string(&self.raw).unwrap_or_else(|_| "{}".into());
        serde_yaml::from_str(&s).unwrap_or(Value::Null)
    }

    /// Serialize back to JSON for `GITHUB_EVENT_PATH` file.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(&self.raw).unwrap_or_else(|_| "{}".into())
    }
}
