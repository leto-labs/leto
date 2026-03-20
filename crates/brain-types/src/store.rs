use std::path::Path;
use std::pin::Pin;

use futures::{Stream, future::BoxFuture};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{BrainError, CredentialEntry, CredentialHealth, Message, Project, ProjectId, Session};

pub type CrudStoreEventStream<E> = Pin<Box<dyn Stream<Item = E> + Send>>;
pub type StoreEventStream = CrudStoreEventStream<StoreEvent>;
pub type ProjectStoreEventStream = CrudStoreEventStream<ProjectStoreEvent>;
pub type SessionStoreEventStream = CrudStoreEventStream<SessionStoreEvent>;
pub type MessageStoreEventStream = CrudStoreEventStream<MessageStoreEvent>;
pub type CredentialStoreEventStream = CrudStoreEventStream<CredentialStoreEvent>;
pub type MessageStoreKey = (Ulid, Ulid);
pub type CredentialStoreKey = (String, String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoreEvent {
    ProjectCreated {
        project: Project,
    },
    ProjectUpdated {
        project: Project,
    },
    ProjectDeleted {
        project_id: ProjectId,
    },
    SessionCreated {
        session: Session,
    },
    SessionUpdated {
        session: Session,
    },
    SessionDeleted {
        session_id: Ulid,
        project_id: ProjectId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectStoreEvent {
    Created { project: Project },
    Updated { project: Project },
    Deleted { project_id: ProjectId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionStoreEvent {
    Created {
        session: Session,
    },
    Updated {
        session: Session,
    },
    Deleted {
        session_id: Ulid,
        project_id: ProjectId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageStoreEvent {
    Created {
        key: MessageStoreKey,
        message: Message,
    },
    Updated {
        key: MessageStoreKey,
        message: Message,
    },
    Deleted {
        key: MessageStoreKey,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialStoreEvent {
    Created {
        key: CredentialStoreKey,
        credential: CredentialEntry,
    },
    Updated {
        key: CredentialStoreKey,
        credential: CredentialEntry,
    },
    Deleted {
        key: CredentialStoreKey,
    },
}

// CrudStore is the shared base for record-oriented stores. It keeps true CRUD
// semantics: create fails on duplicate identity and update replaces an existing
// record, failing if that identity does not exist.
pub trait CrudStore: Send + Sync {
    type Key: Send;
    type Record: Send;
    type Event: Send;

    fn create(
        &self,
        key: Self::Key,
        record: Self::Record,
    ) -> BoxFuture<'_, Result<Self::Record, BrainError>>;

    fn create_many(
        &self,
        entries: Vec<(Self::Key, Self::Record)>,
    ) -> BoxFuture<'_, Result<Vec<Self::Record>, BrainError>> {
        Box::pin(async move {
            let mut created = Vec::with_capacity(entries.len());
            for (key, record) in entries {
                created.push(self.create(key, record).await?);
            }
            Ok(created)
        })
    }

    fn get(&self, key: Self::Key) -> BoxFuture<'_, Result<Self::Record, BrainError>>;

    fn get_many(
        &self,
        keys: Vec<Self::Key>,
    ) -> BoxFuture<'_, Result<Vec<Self::Record>, BrainError>> {
        Box::pin(async move {
            let mut records = Vec::with_capacity(keys.len());
            for key in keys {
                records.push(self.get(key).await?);
            }
            Ok(records)
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Self::Record>, BrainError>>;

    fn update(
        &self,
        key: Self::Key,
        record: Self::Record,
    ) -> BoxFuture<'_, Result<Self::Record, BrainError>>;

    fn update_many(
        &self,
        entries: Vec<(Self::Key, Self::Record)>,
    ) -> BoxFuture<'_, Result<Vec<Self::Record>, BrainError>> {
        Box::pin(async move {
            let mut updated = Vec::with_capacity(entries.len());
            for (key, record) in entries {
                updated.push(self.update(key, record).await?);
            }
            Ok(updated)
        })
    }

    fn delete(&self, key: Self::Key) -> BoxFuture<'_, Result<(), BrainError>>;

    fn delete_many(&self, keys: Vec<Self::Key>) -> BoxFuture<'_, Result<(), BrainError>> {
        Box::pin(async move {
            for key in keys {
                self.delete(key).await?;
            }
            Ok(())
        })
    }

    fn subscribe(&self) -> CrudStoreEventStream<Self::Event>;
}

pub trait ProjectStore:
    CrudStore<Key = ProjectId, Record = Project, Event = ProjectStoreEvent>
{
    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, BrainError>>;
}

pub trait SessionStore: CrudStore<Key = Ulid, Record = Session, Event = SessionStoreEvent> {
    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>>;
}

pub trait MessageStore:
    CrudStore<Key = MessageStoreKey, Record = Message, Event = MessageStoreEvent>
{
    fn list_for_session(&self, session_id: Ulid)
    -> BoxFuture<'_, Result<Vec<Message>, BrainError>>;
}

pub trait CredentialStore:
    CrudStore<Key = CredentialStoreKey, Record = CredentialEntry, Event = CredentialStoreEvent>
{
    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>>;

    fn update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), BrainError>>;
}

pub trait Store: Send + Sync {
    fn projects(&self) -> &dyn ProjectStore;

    fn sessions(&self) -> &dyn SessionStore;

    fn messages(&self) -> &dyn MessageStore;

    fn credentials(&self) -> &dyn CredentialStore;

    fn subscribe(&self) -> StoreEventStream;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;
    use futures::future::ready;

    use super::*;

    #[derive(Default)]
    struct DummyProjectCrudStore {
        created: Mutex<Vec<ProjectId>>,
        fetched: Mutex<Vec<ProjectId>>,
        updated: Mutex<Vec<ProjectId>>,
        deleted: Mutex<Vec<ProjectId>>,
        fail_create: Option<ProjectId>,
        fail_get: Option<ProjectId>,
        fail_update: Option<ProjectId>,
        fail_delete: Option<ProjectId>,
    }

    impl DummyProjectCrudStore {
        fn project(id: ProjectId) -> Project {
            Project {
                id,
                name: Some(format!("project-{id}")),
                root: None,
                config: crate::ProjectConfig::default(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }
    }

    impl CrudStore for DummyProjectCrudStore {
        type Key = ProjectId;
        type Record = Project;
        type Event = ProjectStoreEvent;

        fn create(
            &self,
            key: ProjectId,
            record: Project,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            self.created.lock().unwrap().push(key);
            if self.fail_create == Some(key) {
                return Box::pin(ready(Err(BrainError::Internal("create failed".into()))));
            }
            Box::pin(ready(Ok(record)))
        }

        fn get(&self, key: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
            self.fetched.lock().unwrap().push(key);
            if self.fail_get == Some(key) {
                return Box::pin(ready(Err(BrainError::Internal("get failed".into()))));
            }
            Box::pin(ready(Ok(Self::project(key))))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            key: ProjectId,
            record: Project,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            self.updated.lock().unwrap().push(key);
            if self.fail_update == Some(key) {
                return Box::pin(ready(Err(BrainError::Internal("update failed".into()))));
            }
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, key: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
            self.deleted.lock().unwrap().push(key);
            if self.fail_delete == Some(key) {
                return Box::pin(ready(Err(BrainError::Internal("delete failed".into()))));
            }
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> ProjectStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    struct DummyProjectStore;

    impl CrudStore for DummyProjectStore {
        type Key = ProjectId;
        type Record = Project;
        type Event = ProjectStoreEvent;

        fn create(
            &self,
            _key: ProjectId,
            record: Project,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn get(&self, key: ProjectId) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(DummyProjectCrudStore::project(key))))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: ProjectId,
            record: Project,
        ) -> BoxFuture<'_, Result<Project, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, _key: ProjectId) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> ProjectStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    impl ProjectStore for DummyProjectStore {
        fn find_by_root(&self, _root: &Path) -> BoxFuture<'_, Result<Option<Project>, BrainError>> {
            Box::pin(ready(Ok(None)))
        }
    }

    struct DummySessionStore;

    impl CrudStore for DummySessionStore {
        type Key = Ulid;
        type Record = Session;
        type Event = SessionStoreEvent;

        fn create(
            &self,
            _key: Ulid,
            record: Session,
        ) -> BoxFuture<'_, Result<Session, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn get(&self, key: Ulid) -> BoxFuture<'_, Result<Session, BrainError>> {
            Box::pin(ready(Ok(Session::new(key))))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: Ulid,
            record: Session,
        ) -> BoxFuture<'_, Result<Session, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, _key: Ulid) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> SessionStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    impl SessionStore for DummySessionStore {
        fn list_for_project(
            &self,
            project_id: ProjectId,
        ) -> BoxFuture<'_, Result<Vec<Session>, BrainError>> {
            Box::pin(ready(Ok(vec![Session::new(project_id)])))
        }
    }

    struct DummyMessageStore;

    impl CrudStore for DummyMessageStore {
        type Key = MessageStoreKey;
        type Record = Message;
        type Event = MessageStoreEvent;

        fn create(
            &self,
            _key: MessageStoreKey,
            record: Message,
        ) -> BoxFuture<'_, Result<Message, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn get(&self, key: MessageStoreKey) -> BoxFuture<'_, Result<Message, BrainError>> {
            let message = Message {
                id: key.1,
                role: crate::Role::User,
                content: "dummy".into(),
                tool_calls: Vec::new(),
                tool_call_id: None,
                created_at: Utc::now(),
            };
            Box::pin(ready(Ok(message)))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: MessageStoreKey,
            record: Message,
        ) -> BoxFuture<'_, Result<Message, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, _key: MessageStoreKey) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> MessageStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    impl MessageStore for DummyMessageStore {
        fn list_for_session(
            &self,
            _session_id: Ulid,
        ) -> BoxFuture<'_, Result<Vec<Message>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }
    }

    struct DummyCredentialStore;

    impl CrudStore for DummyCredentialStore {
        type Key = CredentialStoreKey;
        type Record = CredentialEntry;
        type Event = CredentialStoreEvent;

        fn create(
            &self,
            _key: CredentialStoreKey,
            record: CredentialEntry,
        ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn get(
            &self,
            key: CredentialStoreKey,
        ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
            Box::pin(ready(Ok(CredentialEntry::api_key(key.1, "test"))))
        }

        fn list(&self) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update(
            &self,
            _key: CredentialStoreKey,
            record: CredentialEntry,
        ) -> BoxFuture<'_, Result<CredentialEntry, BrainError>> {
            Box::pin(ready(Ok(record)))
        }

        fn delete(&self, _key: CredentialStoreKey) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }

        fn subscribe(&self) -> CredentialStoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    impl CredentialStore for DummyCredentialStore {
        fn list_for_provider(
            &self,
            _provider_name: &str,
        ) -> BoxFuture<'_, Result<Vec<CredentialEntry>, BrainError>> {
            Box::pin(ready(Ok(Vec::new())))
        }

        fn update_health(
            &self,
            _provider_name: &str,
            _credential_id: &str,
            _health: &CredentialHealth,
        ) -> BoxFuture<'_, Result<(), BrainError>> {
            Box::pin(ready(Ok(())))
        }
    }

    struct DummyStore {
        projects: DummyProjectStore,
        sessions: DummySessionStore,
        messages: DummyMessageStore,
        credentials: DummyCredentialStore,
    }

    impl DummyStore {
        fn new() -> Self {
            Self {
                projects: DummyProjectStore,
                sessions: DummySessionStore,
                messages: DummyMessageStore,
                credentials: DummyCredentialStore,
            }
        }
    }

    impl Store for DummyStore {
        fn projects(&self) -> &dyn ProjectStore {
            &self.projects
        }

        fn sessions(&self) -> &dyn SessionStore {
            &self.sessions
        }

        fn messages(&self) -> &dyn MessageStore {
            &self.messages
        }

        fn credentials(&self) -> &dyn CredentialStore {
            &self.credentials
        }

        fn subscribe(&self) -> StoreEventStream {
            Box::pin(futures::stream::empty())
        }
    }

    #[test]
    fn batch_helpers_process_records_in_order() {
        let ids = [Ulid::new(), Ulid::new()];
        let store = DummyProjectCrudStore::default();
        let entries = ids
            .into_iter()
            .map(|id| (id, DummyProjectCrudStore::project(id)))
            .collect::<Vec<_>>();

        let created = futures::executor::block_on(store.create_many(entries.clone())).unwrap();
        let fetched = futures::executor::block_on(store.get_many(ids.to_vec())).unwrap();
        let updated = futures::executor::block_on(store.update_many(entries)).unwrap();
        futures::executor::block_on(store.delete_many(ids.to_vec())).unwrap();

        assert_eq!(created.len(), 2);
        assert_eq!(fetched.len(), 2);
        assert_eq!(updated.len(), 2);
        assert_eq!(*store.created.lock().unwrap(), ids);
        assert_eq!(*store.fetched.lock().unwrap(), ids);
        assert_eq!(*store.updated.lock().unwrap(), ids);
        assert_eq!(*store.deleted.lock().unwrap(), ids);
    }

    #[test]
    fn batch_helpers_fail_fast() {
        let ids = [Ulid::new(), Ulid::new(), Ulid::new()];
        let store = DummyProjectCrudStore {
            fail_create: Some(ids[1]),
            fail_get: Some(ids[1]),
            fail_update: Some(ids[1]),
            fail_delete: Some(ids[1]),
            ..Default::default()
        };
        let entries = ids
            .into_iter()
            .map(|id| (id, DummyProjectCrudStore::project(id)))
            .collect::<Vec<_>>();

        assert!(futures::executor::block_on(store.create_many(entries.clone())).is_err());
        assert!(futures::executor::block_on(store.get_many(ids.to_vec())).is_err());
        assert!(futures::executor::block_on(store.update_many(entries)).is_err());
        assert!(futures::executor::block_on(store.delete_many(ids.to_vec())).is_err());

        assert_eq!(*store.created.lock().unwrap(), vec![ids[0], ids[1]]);
        assert_eq!(*store.fetched.lock().unwrap(), vec![ids[0], ids[1]]);
        assert_eq!(*store.updated.lock().unwrap(), vec![ids[0], ids[1]]);
        assert_eq!(*store.deleted.lock().unwrap(), vec![ids[0], ids[1]]);
    }

    #[test]
    fn composed_store_accessors_are_object_safe() {
        let store = DummyStore::new();
        let dyn_store: &dyn Store = &store;

        let _ = dyn_store.projects();
        let _ = dyn_store.sessions();
        let _ = dyn_store.messages();
        let _ = dyn_store.credentials();
    }

    #[test]
    fn session_and_project_stores_are_usable_through_composed_accessors() {
        let store = DummyStore::new();

        let project = futures::executor::block_on(store.projects().get(Ulid::nil())).unwrap();
        let session = Session::new(project.id);
        let session =
            futures::executor::block_on(store.sessions().create(session.id, session)).unwrap();
        let sessions =
            futures::executor::block_on(store.sessions().list_for_project(project.id)).unwrap();
        let messages =
            futures::executor::block_on(store.messages().list_for_session(session.id)).unwrap();
        let credential = futures::executor::block_on(
            store
                .credentials()
                .get(("dummy-provider".into(), "dummy".into())),
        )
        .unwrap();

        assert_eq!(project.id, Ulid::nil());
        assert_eq!(session.project_id, project.id);
        assert_eq!(sessions.len(), 1);
        assert!(messages.is_empty());
        assert_eq!(credential.id, "dummy");
    }
}
