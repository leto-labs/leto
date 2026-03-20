use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{AgentLoop, BrainError, Provider, Tool};

// A registry owns live runtime objects by name. It deliberately returns the
// stored items directly so callers can decide how to inspect them.
pub trait Registry: Send + Sync {
    type Item: ?Sized;

    fn get(&self, name: &str) -> Option<Arc<Self::Item>>;

    // Listing returns the registry key alongside the live item so callers do
    // not have to assume the item's self-reported name matches the map key.
    fn list(&self) -> Result<Vec<(String, Arc<Self::Item>)>, BrainError>;

    fn set(
        &self,
        name: String,
        item: Arc<Self::Item>,
    ) -> Result<Option<Arc<Self::Item>>, BrainError>;

    fn remove(&self, name: &str) -> Result<Option<Arc<Self::Item>>, BrainError>;
}

pub trait RegistryProvider: Registry<Item = dyn Provider> {}

impl<T> RegistryProvider for T where T: Registry<Item = dyn Provider> + ?Sized {}

pub trait RegistryTool: Registry<Item = dyn Tool> {}

impl<T> RegistryTool for T where T: Registry<Item = dyn Tool> + ?Sized {}

pub trait RegistryLoop: Registry<Item = dyn AgentLoop> {}

impl<T> RegistryLoop for T where T: Registry<Item = dyn AgentLoop> + ?Sized {}

// The default in-memory implementation used by runtimes that want mutable,
// shared registries without exposing the concrete map type in their API.
pub struct RegistryHashMap<T: ?Sized> {
    entries: RwLock<HashMap<String, Arc<T>>>,
}

impl<T: ?Sized> RegistryHashMap<T> {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }
}

impl<T: ?Sized> Default for RegistryHashMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized + Send + Sync> Registry for RegistryHashMap<T> {
    type Item = T;

    fn get(&self, name: &str) -> Option<Arc<Self::Item>> {
        self.entries.read().unwrap().get(name).cloned()
    }

    fn list(&self) -> Result<Vec<(String, Arc<Self::Item>)>, BrainError> {
        Ok(self
            .entries
            .read()
            .unwrap()
            .iter()
            .map(|(name, item)| (name.clone(), Arc::clone(item)))
            .collect())
    }

    fn set(
        &self,
        name: String,
        item: Arc<Self::Item>,
    ) -> Result<Option<Arc<Self::Item>>, BrainError> {
        Ok(self.entries.write().unwrap().insert(name, item))
    }

    fn remove(&self, name: &str) -> Result<Option<Arc<Self::Item>>, BrainError> {
        Ok(self.entries.write().unwrap().remove(name))
    }
}

pub type RegistryProviderHashMap = RegistryHashMap<dyn Provider>;
pub type RegistryToolHashMap = RegistryHashMap<dyn Tool>;
pub type RegistryLoopHashMap = RegistryHashMap<dyn AgentLoop>;
