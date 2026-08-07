//! `action.yml` model (composite, node, docker actions).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub inputs: HashMap<String, ActionInput>,
    #[serde(default)]
    pub outputs: HashMap<String, ActionOutput>,
    #[serde(default)]
    pub runs: Option<ActionRuns>,
    #[serde(default)]
    pub branding: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInput {
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    pub deprecation_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutput {
    pub description: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRuns {
    /// `using:` — `node12`, `node16`, `node20`, `node24`, `docker`, `composite`.
    pub using: ActionRunsUsing,
    /// Composite only — sub-steps.
    #[serde(default)]
    pub steps: Vec<crate::model::Step>,
    /// Node action JS entry points.
    #[serde(default)]
    pub main: Option<String>,
    #[serde(default)]
    pub pre: Option<String>,
    #[serde(default)]
    pub post: Option<String>,
    #[serde(default, rename = "pre-if")]
    pub pre_if: Option<String>,
    #[serde(default, rename = "post-if")]
    pub post_if: Option<String>,
    /// Docker action.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default, rename = "pre-entrypoint")]
    pub pre_entrypoint: Option<String>,
    #[serde(default, rename = "post-entrypoint")]
    pub post_entrypoint: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl ActionRuns {
    pub fn is_node(&self) -> bool {
        matches!(
            self.using,
            ActionRunsUsing::Node12
                | ActionRunsUsing::Node16
                | ActionRunsUsing::Node20
                | ActionRunsUsing::Node24
        )
    }
    pub fn is_docker(&self) -> bool {
        self.using == ActionRunsUsing::Docker
    }
    pub fn is_composite(&self) -> bool {
        self.using == ActionRunsUsing::Composite
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionRunsUsing {
    Node12,
    Node16,
    Node20,
    Node24,
    Docker,
    Composite,
}

impl Default for ActionRuns {
    fn default() -> Self {
        Self {
            using: ActionRunsUsing::Composite,
            steps: Vec::new(),
            main: None,
            pre: None,
            post: None,
            pre_if: Some("always()".to_string()),
            post_if: Some("always()".to_string()),
            image: None,
            entrypoint: None,
            pre_entrypoint: None,
            post_entrypoint: None,
            args: Vec::new(),
            env: HashMap::new(),
        }
    }
}

/// Read an `action.yml` / `action.yaml` from the given directory.
pub fn read_action(action_dir: &std::path::Path) -> anyhow::Result<Action> {
    for name in ["action.yml", "action.yaml"] {
        let path = action_dir.join(name);
        if path.is_file() {
            let bytes = std::fs::read(&path)?;
            let mut action: Action = serde_yaml::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("invalid action.yml at {}: {e}", path.display()))?;
            if action.runs.is_none() {
                action.runs = Some(ActionRuns::default());
            } else if let Some(r) = &mut action.runs {
                if r.pre_if.is_none() {
                    r.pre_if = Some("always()".to_string());
                }
                if r.post_if.is_none() {
                    r.post_if = Some("always()".to_string());
                }
            }
            return Ok(action);
        }
    }
    anyhow::bail!("no action.yml or action.yaml found in {}", action_dir.display());
}
