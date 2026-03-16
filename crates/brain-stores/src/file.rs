use std::path::{Path, PathBuf};

use chrono::Utc;
use futures::future::BoxFuture;
use ulid::Ulid;

use brain_types::*;

pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, BrainError> {
        let root = root.into();
        tokio::fs::create_dir_all(root.join("projects"))
            .await
            .map_err(|e| BrainError::Storage(format!("failed to create projects dir: {e}")))?;
        tokio::fs::create_dir_all(root.join("sessions"))
            .await
            .map_err(|e| BrainError::Storage(format!("failed to create sessions dir: {e}")))?;
        tokio::fs::create_dir_all(root.join("credentials"))
            .await
            .map_err(|e| BrainError::Storage(format!("failed to create credentials dir: {e}")))?;
        Ok(Self { root })
    }

    fn project_file(&self, id: ProjectId) -> PathBuf {
        self.root.join("projects").join(format!("{id}.json"))
    }

    fn session_dir(&self, session_id: Ulid) -> PathBuf {
        self.root.join("sessions").join(session_id.to_string())
    }

    fn session_file(&self, session_id: Ulid) -> PathBuf {
        self.session_dir(session_id).join("session.json")
    }

    fn messages_file(&self, session_id: Ulid) -> PathBuf {
        self.session_dir(session_id).join("messages.jsonl")
    }

    fn credentials_dir(&self) -> PathBuf {
        self.root.join("credentials")
    }

    fn credential_provider_dir(&self, provider_name: &str) -> PathBuf {
        self.credentials_dir().join(provider_name)
    }

    fn credential_file(&self, provider_name: &str, credential_id: &str) -> PathBuf {
        self.credential_provider_dir(provider_name)
            .join(format!("{credential_id}.json"))
    }
}

async fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, BrainError> {
    let data = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| BrainError::Storage(format!("read {}: {e}", path.display())))?;
    serde_json::from_str(&data)
        .map_err(|e| BrainError::Storage(format!("parse {}: {e}", path.display())))
}

async fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), BrainError> {
    let data = serde_json::to_string_pretty(value)
        .map_err(|e| BrainError::Storage(format!("serialize: {e}")))?;
    tokio::fs::write(path, data)
        .await
        .map_err(|e| BrainError::Storage(format!("write {}: {e}", path.display())))
}

impl ProjectStore for FileStore {
    fn project_create(&self, project: Project) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move {
            write_json(&self.project_file(project.id), &project).await?;
            Ok(project)
        })
    }

    fn project_get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
        Box::pin(async move {
            let path = self.project_file(id);
            if !path.exists() {
                return Err(BrainError::Storage(format!("project not found: {id}")));
            }
            read_json(&path).await
        })
    }

    fn project_list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
        Box::pin(async move {
            let projects_dir = self.root.join("projects");
            let mut entries = tokio::fs::read_dir(&projects_dir)
                .await
                .map_err(|e| BrainError::Storage(format!("read projects dir: {e}")))?;

            let mut projects = Vec::new();
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match read_json::<Project>(&path).await {
                    Ok(p) => projects.push(p),
                    Err(_) => continue,
                }
            }
            projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(projects)
        })
    }

    fn project_update(&self, id: ProjectId, update: ProjectUpdate) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let path = self.project_file(id);
            let mut project: Project = read_json(&path).await?;
            if let Some(name) = update.name {
                project.name = Some(name);
            }
            if let Some(config) = update.config {
                project.config = config;
            }
            project.updated_at = Utc::now();
            write_json(&path, &project).await
        })
    }

    fn project_delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let path = self.project_file(id);
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(BrainError::Storage(format!("delete project: {e}"))),
            }
        })
    }
}

impl SessionStore for FileStore {
    fn session_create(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move {
            let session = Session::new(project_id);
            let dir = self.session_dir(session.id);
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| BrainError::Storage(format!("create session dir: {e}")))?;
            write_json(&self.session_file(session.id), &session).await?;
            tokio::fs::write(self.messages_file(session.id), "")
                .await
                .map_err(|e| BrainError::Storage(format!("create messages file: {e}")))?;
            Ok(session)
        })
    }

    fn session_get(&self, id: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
        Box::pin(async move {
            let path = self.session_file(id);
            if !path.exists() {
                return Err(BrainError::Storage(format!("session not found: {id}")));
            }
            read_json(&path).await
        })
    }

    fn session_list(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
        Box::pin(async move {
            let sessions_dir = self.root.join("sessions");
            if !sessions_dir.exists() {
                return Ok(Vec::new());
            }

            let mut entries = tokio::fs::read_dir(&sessions_dir)
                .await
                .map_err(|e| BrainError::Storage(format!("read sessions dir: {e}")))?;

            let mut sessions = Vec::new();
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let session_file = entry.path().join("session.json");
                if !session_file.exists() {
                    continue;
                }
                match read_json::<Session>(&session_file).await {
                    Ok(s) if s.project_id == project_id => sessions.push(s),
                    _ => continue,
                }
            }
            sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(sessions)
        })
    }

    fn session_update(&self, id: Ulid, update: SessionUpdate) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let path = self.session_file(id);
            let mut session: Session = read_json(&path).await?;
            if let Some(title) = update.title {
                session.title = Some(title);
            }
            session.updated_at = Utc::now();
            write_json(&path, &session).await
        })
    }

    fn session_delete(&self, id: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            let dir = self.session_dir(id);
            if dir.exists() {
                tokio::fs::remove_dir_all(&dir)
                    .await
                    .map_err(|e| BrainError::Storage(format!("delete session: {e}")))?;
            }
            Ok(())
        })
    }
}

impl MessageStore for FileStore {
    fn message_append(&self, session_id: Ulid, msgs: &[Message]) -> BoxFuture<'_, Result<(), BrainError>> {
        let msgs = msgs.to_vec();
        Box::pin(async move {
            let path = self.messages_file(session_id);

            let mut content = String::new();
            for msg in &msgs {
                let line = serde_json::to_string(msg)
                    .map_err(|e| BrainError::Storage(format!("serialize message: {e}")))?;
                content.push_str(&line);
                content.push('\n');
            }

            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| BrainError::Storage(format!("open messages file: {e}")))?;
            file.write_all(content.as_bytes())
                .await
                .map_err(|e| BrainError::Storage(format!("write messages: {e}")))?;
            file.flush()
                .await
                .map_err(|e| BrainError::Storage(format!("flush messages: {e}")))?;

            let session_path = self.session_file(session_id);
            if let Ok(mut s) = read_json::<Session>(&session_path).await {
                s.updated_at = Utc::now();
                let _ = write_json(&session_path, &s).await;
            }

            Ok(())
        })
    }

    fn message_list(&self, session_id: Ulid) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
        Box::pin(async move {
            let path = self.messages_file(session_id);

            if !path.exists() {
                return Ok(Vec::new());
            }
            let data = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| BrainError::Storage(format!("read messages: {e}")))?;

            let mut messages = Vec::new();
            for line in data.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let msg: Message = serde_json::from_str(line)
                    .map_err(|e| BrainError::Storage(format!("parse message line: {e}")))?;
                messages.push(msg);
            }
            Ok(messages)
        })
    }
}

impl CredentialStore for FileStore {
    fn credential_save(&self, provider_name: &str, entry: &CredentialEntry) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        let entry = entry.clone();
        Box::pin(async move {
            let dir = self.credential_provider_dir(&name);
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| BrainError::Storage(format!("create credentials dir: {e}")))?;

            let path = self.credential_file(&name, &entry.id);
            write_json(&path, &entry).await?;
            set_file_permissions_restrictive(&path).await?;
            Ok(())
        })
    }

    fn credential_load(&self, provider_name: &str, credential_id: &str) -> BoxFuture<'_, Result<Option<CredentialEntry>, BrainError>> {
        let name = provider_name.to_owned();
        let cid = credential_id.to_owned();
        Box::pin(async move {
            let path = self.credential_file(&name, &cid);
            match tokio::fs::read_to_string(&path).await {
                Ok(data) => {
                    let entry: CredentialEntry = serde_json::from_str(&data)
                        .map_err(|e| BrainError::Storage(format!(
                            "corrupt credential file {}: {e}", path.display()
                        )))?;
                    Ok(Some(entry))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(BrainError::Storage(format!(
                    "read credential {}: {e}", path.display()
                ))),
            }
        })
    }

    fn credential_load_all(&self, provider_name: &str) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
        let name = provider_name.to_owned();
        Box::pin(async move {
            let dir = self.credential_provider_dir(&name);
            if !dir.exists() {
                return Ok(Vec::new());
            }

            let mut dir_entries = tokio::fs::read_dir(&dir)
                .await
                .map_err(|e| BrainError::Storage(format!("read credentials dir: {e}")))?;

            let mut entries = Vec::new();
            while let Some(de) = dir_entries
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let path = de.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match read_json::<CredentialEntry>(&path).await {
                    Ok(entry) => entries.push(entry),
                    Err(_) => continue,
                }
            }
            Ok(entries)
        })
    }

    fn credential_delete(&self, provider_name: &str, credential_id: &str) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        let cid = credential_id.to_owned();
        Box::pin(async move {
            let path = self.credential_file(&name, &cid);
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(BrainError::Storage(format!(
                    "delete credential {}: {e}", path.display()
                ))),
            }
        })
    }

    fn credential_update_health(&self, provider_name: &str, credential_id: &str, health: &CredentialHealth) -> BoxFuture<'_, Result<(), BrainError>> {
        let name = provider_name.to_owned();
        let cid = credential_id.to_owned();
        let health = health.clone();
        Box::pin(async move {
            let path = self.credential_file(&name, &cid);
            match tokio::fs::read_to_string(&path).await {
                Ok(data) => {
                    let mut entry: CredentialEntry = serde_json::from_str(&data)
                        .map_err(|e| BrainError::Storage(format!(
                            "corrupt credential file {}: {e}", path.display()
                        )))?;
                    entry.health = health;
                    write_json(&path, &entry).await?;
                    Ok(())
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(BrainError::Storage(format!(
                    "read credential {}: {e}", path.display()
                ))),
            }
        })
    }

    fn credential_list(&self) -> BoxFuture<'_, Result<Vec<(String, CredentialEntry)>, BrainError>> {
        Box::pin(async move {
            let dir = self.credentials_dir();
            if !dir.exists() {
                return Ok(Vec::new());
            }

            let mut provider_dirs = tokio::fs::read_dir(&dir)
                .await
                .map_err(|e| BrainError::Storage(format!("read credentials dir: {e}")))?;

            let mut creds = Vec::new();
            while let Some(provider_entry) = provider_dirs
                .next_entry()
                .await
                .map_err(|e| BrainError::Storage(format!("read dir entry: {e}")))?
            {
                let provider_path = provider_entry.path();
                if !provider_path.is_dir() {
                    continue;
                }
                let provider_name = provider_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_owned();

                let mut cred_files = match tokio::fs::read_dir(&provider_path).await {
                    Ok(rd) => rd,
                    Err(_) => continue,
                };
                while let Ok(Some(cred_entry)) = cred_files.next_entry().await {
                    let path = cred_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    match read_json::<CredentialEntry>(&path).await {
                        Ok(entry) => creds.push((provider_name.clone(), entry)),
                        Err(_) => continue,
                    }
                }
            }
            Ok(creds)
        })
    }
}

#[cfg(unix)]
async fn set_file_permissions_restrictive(path: &Path) -> Result<(), BrainError> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    tokio::fs::set_permissions(path, perms)
        .await
        .map_err(|e| BrainError::Storage(format!(
            "set permissions on {}: {e}", path.display()
        )))
}

#[cfg(not(unix))]
async fn set_file_permissions_restrictive(_path: &Path) -> Result<(), BrainError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api_key_entry(id: &str, key: &str) -> CredentialEntry {
        CredentialEntry::api_key(id, key)
    }

    async fn temp_store() -> (FileStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path()).await.unwrap();
        (store, dir)
    }

    // ── ProjectStore ───────────────────────────────────────────

    #[tokio::test]
    async fn project_crud() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("test");
        let id = project.id;

        let created = store.project_create(project).await.unwrap();
        assert_eq!(created.name.as_deref(), Some("test"));

        let fetched = store.project_get(id).await.unwrap();
        assert_eq!(fetched.id, id);
        assert_eq!(fetched.name.as_deref(), Some("test"));

        let list = store.project_list().await.unwrap();
        assert_eq!(list.len(), 1);

        store
            .project_update(id, ProjectUpdate { name: Some("renamed".into()), config: None })
            .await
            .unwrap();
        let updated = store.project_get(id).await.unwrap();
        assert_eq!(updated.name.as_deref(), Some("renamed"));

        store.project_delete(id).await.unwrap();
        assert!(store.project_get(id).await.is_err());
    }

    #[tokio::test]
    async fn project_get_not_found() {
        let (store, _dir) = temp_store().await;
        assert!(store.project_get(Ulid::new()).await.is_err());
    }

    #[tokio::test]
    async fn project_list_multiple() {
        let (store, _dir) = temp_store().await;
        store.project_create(Project::with_defaults("a")).await.unwrap();
        store.project_create(Project::with_defaults("b")).await.unwrap();
        let list = store.project_list().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn project_delete_idempotent() {
        let (store, _dir) = temp_store().await;
        store.project_delete(Ulid::new()).await.unwrap();
    }

    // ── SessionStore ───────────────────────────────────────────

    #[tokio::test]
    async fn session_crud() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let session = store.session_create(pid).await.unwrap();
        let sid = session.id;
        assert_eq!(session.project_id, pid);

        let fetched = store.session_get(sid).await.unwrap();
        assert_eq!(fetched.id, sid);

        let list = store.session_list(pid).await.unwrap();
        assert_eq!(list.len(), 1);

        store
            .session_update(sid, SessionUpdate { title: Some("My Chat".into()) })
            .await
            .unwrap();
        let updated = store.session_get(sid).await.unwrap();
        assert_eq!(updated.title.as_deref(), Some("My Chat"));

        store.session_delete(sid).await.unwrap();
        assert!(store.session_get(sid).await.is_err());
    }

    #[tokio::test]
    async fn session_list_empty_for_new_project() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let list = store.session_list(pid).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn session_list_filters_by_project() {
        let (store, _dir) = temp_store().await;
        let p1 = Project::with_defaults("proj1");
        let p2 = Project::with_defaults("proj2");
        let pid1 = p1.id;
        let pid2 = p2.id;
        store.project_create(p1).await.unwrap();
        store.project_create(p2).await.unwrap();

        store.session_create(pid1).await.unwrap();
        store.session_create(pid1).await.unwrap();
        store.session_create(pid2).await.unwrap();

        let list1 = store.session_list(pid1).await.unwrap();
        assert_eq!(list1.len(), 2);

        let list2 = store.session_list(pid2).await.unwrap();
        assert_eq!(list2.len(), 1);
    }

    #[tokio::test]
    async fn session_list_nonexistent_project() {
        let (store, _dir) = temp_store().await;
        let list = store.session_list(Ulid::new()).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn session_get_not_found() {
        let (store, _dir) = temp_store().await;
        assert!(store.session_get(Ulid::new()).await.is_err());
    }

    // ── MessageStore ───────────────────────────────────────────

    #[tokio::test]
    async fn message_append_and_list() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let session = store.session_create(pid).await.unwrap();
        let sid = session.id;

        let empty = store.message_list(sid).await.unwrap();
        assert!(empty.is_empty());

        let msgs = vec![Message::user("hello"), Message::assistant("hi")];
        store.message_append(sid, &msgs).await.unwrap();

        let loaded = store.message_list(sid).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "hello");
        assert_eq!(loaded[1].content, "hi");
    }

    #[tokio::test]
    async fn message_append_is_additive() {
        let (store, _dir) = temp_store().await;
        let project = Project::with_defaults("proj");
        let pid = project.id;
        store.project_create(project).await.unwrap();

        let session = store.session_create(pid).await.unwrap();
        let sid = session.id;

        store.message_append(sid, &[Message::user("first")]).await.unwrap();
        store.message_append(sid, &[Message::user("second")]).await.unwrap();

        let loaded = store.message_list(sid).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "first");
        assert_eq!(loaded[1].content, "second");
    }

    // ── CredentialStore ────────────────────────────────────────

    #[tokio::test]
    async fn credential_crud() {
        let (store, _dir) = temp_store().await;

        assert!(store.credential_load("openai", "key-1").await.unwrap().is_none());

        store.credential_save("openai", &api_key_entry("key-1", "sk-123")).await.unwrap();

        let loaded = store.credential_load("openai", "key-1").await.unwrap().unwrap();
        assert_eq!(loaded.id, "key-1");
        match &loaded.credential {
            ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "sk-123"),
            _ => panic!("expected ApiKey"),
        }

        let list = store.credential_list().await.unwrap();
        assert_eq!(list.len(), 1);

        store.credential_delete("openai", "key-1").await.unwrap();
        assert!(store.credential_load("openai", "key-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn credential_multi_per_provider() {
        let (store, _dir) = temp_store().await;
        store.credential_save("openai", &api_key_entry("key-1", "sk-111")).await.unwrap();
        store.credential_save("openai", &api_key_entry("key-2", "sk-222")).await.unwrap();

        let all = store.credential_load_all("openai").await.unwrap();
        assert_eq!(all.len(), 2);

        store.credential_delete("openai", "key-1").await.unwrap();
        let all = store.credential_load_all("openai").await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "key-2");
    }

    #[tokio::test]
    async fn credential_delete_idempotent() {
        let (store, _dir) = temp_store().await;
        store.credential_delete("nonexistent", "nope").await.unwrap();
    }

    #[tokio::test]
    async fn credential_list_empty_when_no_creds() {
        let (store, _dir) = temp_store().await;
        let list = store.credential_list().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn credential_overwrite() {
        let (store, _dir) = temp_store().await;
        store.credential_save("p", &api_key_entry("k", "old")).await.unwrap();
        store.credential_save("p", &api_key_entry("k", "new")).await.unwrap();

        let loaded = store.credential_load("p", "k").await.unwrap().unwrap();
        match &loaded.credential {
            ProviderCredential::ApiKey { api_key } => assert_eq!(api_key, "new"),
            _ => panic!("expected ApiKey"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn credential_file_has_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let (store, _dir) = temp_store().await;
        store.credential_save("secret", &api_key_entry("key-1", "sk-x")).await.unwrap();

        let path = store.credential_file("secret", "key-1");
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
}
