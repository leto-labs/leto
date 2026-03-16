use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::config::AgentConfig;

pub type ProjectId = Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(flatten)]
    pub agent: AgentConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
        }
    }
}

impl ProjectConfig {
    pub fn agent_config(&self) -> &AgentConfig {
        &self.agent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: Option<String>,
    pub root: Option<PathBuf>,
    pub config: ProjectConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: Option<String>, root: Option<PathBuf>, config: ProjectConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Ulid::new(),
            name,
            root,
            config,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_defaults(name: &str) -> Self {
        Self::new(Some(name.to_owned()), None, ProjectConfig::default())
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new(None, None, ProjectConfig::default())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub config: Option<ProjectConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_defaults_sets_name() {
        let p = Project::with_defaults("myproject");
        assert_eq!(p.name.as_deref(), Some("myproject"));
        assert!(p.root.is_none());
        assert_eq!(p.config.agent.max_iterations, 20);
    }

    #[test]
    fn default_project_has_no_name() {
        let p = Project::default();
        assert!(p.name.is_none());
        assert!(p.root.is_none());
    }

    #[test]
    fn project_config_agent_accessor() {
        let cfg = ProjectConfig::default();
        assert_eq!(cfg.agent_config().max_iterations, 20);
    }

    #[test]
    fn project_serde_roundtrip() {
        let p = Project::with_defaults("test");
        let json = serde_json::to_string(&p).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, p.id);
        assert_eq!(deserialized.name, p.name);
    }

    #[test]
    fn project_update_default() {
        let update = ProjectUpdate::default();
        assert!(update.name.is_none());
        assert!(update.config.is_none());
    }
}
