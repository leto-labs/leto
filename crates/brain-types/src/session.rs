use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::ProjectId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Ulid,
    pub project_id: ProjectId,
    pub title: Option<String>,
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
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_has_no_title() {
        let s = Session::new(Ulid::new());
        assert!(s.title.is_none());
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
    }
}
