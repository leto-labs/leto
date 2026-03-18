use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use futures::future::BoxFuture;
use tokio::sync::RwLock;
use ulid::Ulid;

use brain_types::*;

struct SessionData {
    session: Session,
    messages: Vec<Message>,
}

pub struct InMemoryStore {
    projects: Arc<RwLock<HashMap<ProjectId, Project>>>,
    sessions: Arc<RwLock<HashMap<Ulid, SessionData>>>,
    credentials: Arc<RwLock<HashMap<(String, String), CredentialEntry>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            projects: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectStore for InMemoryStore {
    fn project_create(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move {
            let mut projects = self.projects.write().await;
            projects.insert(project.id, project.clone());
            Ok(project)
        })
    }

    fn project_get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move {
            let projects = self.projects.read().await;
            projects
                .get(&id)
                .cloned()
                .ok_or_else(|| BrainError::Storage(format!("project not found: {id}")))
        })
    }

    fn project_list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        Box::pin(async move {
            let projects = self.projects.read().await;
            let mut list: Vec<Project> = projects.values().cloned().collect();
            list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(list)
        })
    }

    fn project_update(
        &self,
        id: ProjectId,
        update: ProjectUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let mut projects = self.projects.write().await;
            let project = projects
                .get_mut(&id)
                .ok_or_else(|| BrainError::Storage(format!("project not found: {id}")))?;
            if let Some(name) = update.name {
                project.name = Some(name);
            }
            if let Some(config) = update.config {
                project.config = config;
            }
            project.updated_at = Utc::now();
            Ok(())
        })
    }

    fn project_delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let mut projects = self.projects.write().await;
            projects.remove(&id);
            let mut sessions = self.sessions.write().await;
            sessions.retain(|_, data| data.session.project_id != id);
            Ok(())
        })
    }
}

impl SessionStore for InMemoryStore {
    fn session_create(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move {
            let session = Session::new(project_id);
            let mut sessions = self.sessions.write().await;
            sessions.insert(
                session.id,
                SessionData {
                    session: session.clone(),
                    messages: Vec::new(),
                },
            );
            Ok(session)
        })
    }

    fn session_get(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move {
            let sessions = self.sessions.read().await;
            sessions
                .get(&id)
                .map(|d| d.session.clone())
                .ok_or_else(|| BrainError::Storage(format!("session not found: {id}")))
        })
    }

    fn session_list(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        Box::pin(async move {
            let sessions = self.sessions.read().await;
            let mut list: Vec<Session> = sessions
                .values()
                .filter(|d| d.session.project_id == project_id)
                .map(|d| d.session.clone())
                .collect();
            list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(list)
        })
    }

    fn session_update(
        &self,
        id: Ulid,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let mut sessions = self.sessions.write().await;
            let entry = sessions
                .get_mut(&id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {id}")))?;
            if let Some(title) = update.title {
                entry.session.title = Some(title);
            }
            entry.session.updated_at = Utc::now();
            Ok(())
        })
    }

    fn session_delete(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let mut sessions = self.sessions.write().await;
            sessions.remove(&id);
            Ok(())
        })
    }
}

impl MessageStore for InMemoryStore {
    fn message_append(
        &self,
        session_id: Ulid,
        msgs: &[Message],
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let msgs = msgs.to_vec();
        Box::pin(async move {
            let mut sessions = self.sessions.write().await;
            let entry = sessions
                .get_mut(&session_id)
                .ok_or_else(|| BrainError::Storage(format!("session not found: {session_id}")))?;
            entry.messages.extend(msgs);
            entry.session.updated_at = Utc::now();
            Ok(())
        })
    }

    fn message_list(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        Box::pin(async move {
            let sessions = self.sessions.read().await;
            Ok(sessions
                .get(&session_id)
                .map(|d| d.messages.clone())
                .unwrap_or_default())
        })
    }
}

impl CredentialStore for InMemoryStore {
    fn credential_save(
        &self,
        provider_name: &str,
        entry: &CredentialEntry,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let key = (provider_name.to_owned(), entry.id.clone());
        let entry = entry.clone();
        Box::pin(async move {
            self.credentials.write().await.insert(key, entry);
            Ok(())
        })
    }

    fn credential_load(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<Option<CredentialEntry>, BrainError>> {
        let key = (provider_name.to_owned(), credential_id.to_owned());
        Box::pin(async move { Ok(self.credentials.read().await.get(&key).cloned()) })
    }

    fn credential_load_all(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move {
            let creds = self.credentials.read().await;
            Ok(creds
                .iter()
                .filter(|((pn, _), _)| *pn == name)
                .map(|(_, v)| v.clone())
                .collect())
        })
    }

    fn credential_delete(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let key = (provider_name.to_owned(), credential_id.to_owned());
        Box::pin(async move {
            self.credentials.write().await.remove(&key);
            Ok(())
        })
    }

    fn credential_update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), BrainError>> {
        let key = (provider_name.to_owned(), credential_id.to_owned());
        let health = health.clone();
        Box::pin(async move {
            let mut creds = self.credentials.write().await;
            if let Some(entry) = creds.get_mut(&key) {
                entry.health = health;
            }
            Ok(())
        })
    }

    fn credential_list(&self) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>> {
        Box::pin(async move {
            let creds = self.credentials.read().await;
            Ok(creds
                .iter()
                .map(|((pn, _), v)| (pn.clone(), v.clone()))
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api_key_entry(id: &str, key: &str) -> CredentialEntry {
        CredentialEntry::api_key(id, key)
    }

    // ── ProjectStore ───────────────────────────────────────────

    #[tokio::test]
    async fn project_crud() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("test");
        let id = project.id;

        let created = store.project_create(project).await.unwrap();
        assert_eq!(created.name.as_deref(), Some("test"));

        let fetched = store.project_get(id).await.unwrap();
        assert_eq!(fetched.id, id);

        let list = store.project_list().await.unwrap();
        assert_eq!(list.len(), 1);

        store
            .project_update(
                id,
                ProjectUpdate {
                    name: Some("renamed".into()),
                    config: None,
                },
            )
            .await
            .unwrap();
        let updated = store.project_get(id).await.unwrap();
        assert_eq!(updated.name.as_deref(), Some("renamed"));

        store.project_delete(id).await.unwrap();
        assert!(store.project_get(id).await.is_err());
    }

    #[tokio::test]
    async fn project_get_not_found() {
        let store = InMemoryStore::new();
        assert!(store.project_get(Ulid::new()).await.is_err());
    }

    #[tokio::test]
    async fn project_list_sorted_by_updated_at() {
        let store = InMemoryStore::new();
        let p1 = Project::with_defaults("first");
        let id1 = p1.id;
        store.project_create(p1).await.unwrap();

        let p2 = Project::with_defaults("second");
        store.project_create(p2).await.unwrap();

        // Update p1 so its updated_at is newest
        store
            .project_update(
                id1,
                ProjectUpdate {
                    name: Some("first-updated".into()),
                    config: None,
                },
            )
            .await
            .unwrap();

        let list = store.project_list().await.unwrap();
        assert_eq!(list[0].id, id1);
    }

    #[tokio::test]
    async fn project_delete_cascades_sessions() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let session = store.session_create(pid).await.unwrap();
        let sid = session.id;

        store.project_delete(pid).await.unwrap();
        assert!(store.session_get(sid).await.is_err());
    }

    // ── SessionStore ───────────────────────────────────────────

    #[tokio::test]
    async fn session_crud() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let session = store.session_create(pid).await.unwrap();
        let sid = session.id;
        assert_eq!(session.project_id, pid);
        assert!(session.title.is_none());

        let fetched = store.session_get(sid).await.unwrap();
        assert_eq!(fetched.id, sid);

        let list = store.session_list(pid).await.unwrap();
        assert_eq!(list.len(), 1);

        store
            .session_update(
                sid,
                SessionUpdate {
                    title: Some("My Chat".into()),
                },
            )
            .await
            .unwrap();
        let updated = store.session_get(sid).await.unwrap();
        assert_eq!(updated.title.as_deref(), Some("My Chat"));

        store.session_delete(sid).await.unwrap();
        assert!(store.session_get(sid).await.is_err());
    }

    #[tokio::test]
    async fn session_list_filters_by_project() {
        let store = InMemoryStore::new();
        let p1 = Project::with_defaults("p1");
        let p2 = Project::with_defaults("p2");
        let pid1 = p1.id;
        let pid2 = p2.id;
        store.project_create(p1).await.unwrap();
        store.project_create(p2).await.unwrap();

        store.session_create(pid1).await.unwrap();
        store.session_create(pid1).await.unwrap();
        store.session_create(pid2).await.unwrap();

        assert_eq!(store.session_list(pid1).await.unwrap().len(), 2);
        assert_eq!(store.session_list(pid2).await.unwrap().len(), 1);
    }

    // ── MessageStore ───────────────────────────────────────────

    #[tokio::test]
    async fn message_append_and_list() {
        let store = InMemoryStore::new();
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let session = store.session_create(pid).await.unwrap();
        let sid = session.id;

        assert!(store.message_list(sid).await.unwrap().is_empty());

        let msgs = vec![Message::user("hello"), Message::assistant("hi there")];
        store.message_append(sid, &msgs).await.unwrap();

        let loaded = store.message_list(sid).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, Role::User);
        assert_eq!(loaded[0].content, "hello");
        assert_eq!(loaded[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn message_list_returns_empty_for_missing_session() {
        let store = InMemoryStore::new();
        let list = store.message_list(Ulid::new()).await.unwrap();
        assert!(list.is_empty());
    }

    // ── CredentialStore ────────────────────────────────────────

    #[tokio::test]
    async fn credential_crud() {
        let store = InMemoryStore::new();

        assert!(
            store
                .credential_load("openai", "key-1")
                .await
                .unwrap()
                .is_none()
        );

        let entry = api_key_entry("key-1", "sk-123");
        store.credential_save("openai", &entry).await.unwrap();

        let loaded = store
            .credential_load("openai", "key-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.id, "key-1");
        match &loaded.credential {
            ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-123"),
            _ => panic!("expected ApiKey"),
        }

        let list = store.credential_list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "openai");

        store.credential_delete("openai", "key-1").await.unwrap();
        assert!(
            store
                .credential_load("openai", "key-1")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn credential_multi_per_provider() {
        let store = InMemoryStore::new();
        store
            .credential_save("openai", &api_key_entry("key-1", "sk-111"))
            .await
            .unwrap();
        store
            .credential_save("openai", &api_key_entry("key-2", "sk-222"))
            .await
            .unwrap();

        let all = store.credential_load_all("openai").await.unwrap();
        assert_eq!(all.len(), 2);

        store.credential_delete("openai", "key-1").await.unwrap();
        let all = store.credential_load_all("openai").await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "key-2");
    }

    #[tokio::test]
    async fn credential_overwrite() {
        let store = InMemoryStore::new();
        store
            .credential_save("p", &api_key_entry("k", "old"))
            .await
            .unwrap();
        store
            .credential_save("p", &api_key_entry("k", "new"))
            .await
            .unwrap();

        let loaded = store.credential_load("p", "k").await.unwrap().unwrap();
        match &loaded.credential {
            ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "new"),
            _ => panic!("expected ApiKey"),
        }
    }

    #[tokio::test]
    async fn credential_update_health() {
        let store = InMemoryStore::new();
        store
            .credential_save("openai", &api_key_entry("key-1", "sk-123"))
            .await
            .unwrap();

        let mut health = CredentialHealth::default();
        health.record_error("rate limited", Some("429".into()));
        store
            .credential_update_health("openai", "key-1", &health)
            .await
            .unwrap();

        let loaded = store
            .credential_load("openai", "key-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.health.consecutive_errors, 1);
        assert_eq!(
            loaded.health.last_error.as_ref().unwrap().code.as_deref(),
            Some("429")
        );
    }

    #[tokio::test]
    async fn default_impl() {
        let store = InMemoryStore::default();
        assert!(store.project_list().await.unwrap().is_empty());
    }
}
