use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{InferenceConfig, ProjectId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Ulid,
    pub project_id: ProjectId,
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    pub fn new(project_id: ProjectId) -> Self {
        let now = Utc::now();
        Self {
            id: Ulid::new(),
            project_id,
            title: None,
            inference: None,
            loop_name: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<SessionInferenceUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_name: Option<SessionLoopUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionInferenceUpdate {
    Set(InferenceConfig),
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLoopUpdate {
    Set(String),
    Clear,
}

impl SessionUpdate {
    pub fn title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            inference: None,
            loop_name: None,
        }
    }

    pub fn inference(inference: InferenceConfig) -> Self {
        Self {
            title: None,
            inference: Some(SessionInferenceUpdate::Set(inference)),
            loop_name: None,
        }
    }

    pub fn clear_inference() -> Self {
        Self {
            title: None,
            inference: Some(SessionInferenceUpdate::Clear),
            loop_name: None,
        }
    }

    pub fn loop_name(loop_name: impl Into<String>) -> Self {
        Self {
            title: None,
            inference: None,
            loop_name: Some(SessionLoopUpdate::Set(loop_name.into())),
        }
    }

    pub fn clear_loop_name() -> Self {
        Self {
            title: None,
            inference: None,
            loop_name: Some(SessionLoopUpdate::Clear),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_has_no_title() {
        let s = Session::new(Ulid::new());
        assert!(s.title.is_none());
        assert!(s.inference.is_none());
        assert!(s.loop_name.is_none());
        assert!(s.created_at <= Utc::now());
    }

    #[test]
    fn session_stores_project_id() {
        let pid = Ulid::new();
        let s = Session::new(pid);
        assert_eq!(s.project_id, pid);
    }

    #[test]
    fn session_serde_roundtrip() {
        let s = Session::new(Ulid::new());
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, s.id);
        assert_eq!(deserialized.project_id, s.project_id);
    }

    #[test]
    fn session_update_default() {
        let u = SessionUpdate::default();
        assert!(u.title.is_none());
        assert!(u.inference.is_none());
        assert!(u.loop_name.is_none());
    }

    #[test]
    fn session_update_builder_sets_inference() {
        let update = SessionUpdate::inference(InferenceConfig {
            provider: Some("openai".into()),
            model: Some("gpt-5".into()),
            max_tokens: None,
            temperature: None,
        });

        match update.inference {
            Some(SessionInferenceUpdate::Set(config)) => {
                assert_eq!(config.provider.as_deref(), Some("openai"));
                assert_eq!(config.model.as_deref(), Some("gpt-5"));
            }
            _ => panic!("expected inference set update"),
        }
    }

    #[test]
    fn session_update_builder_sets_loop_name() {
        let update = SessionUpdate::loop_name("simple");

        match update.loop_name {
            Some(SessionLoopUpdate::Set(loop_name)) => {
                assert_eq!(loop_name, "simple");
            }
            _ => panic!("expected loop name set update"),
        }
    }
}
