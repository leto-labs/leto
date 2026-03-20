use brain_types::{
    AgentLoop, BrainRuntime, CredentialStore, CredentialStoreEvent, CredentialStoreKey, CrudStore,
    EventStream, MessageStore, MessageStoreEvent, MessageStoreKey, ProjectStore, ProjectStoreEvent,
    Registry, RegistryHashMap, RegistryLoop, RegistryLoopHashMap, RegistryProvider,
    RegistryProviderHashMap, RegistryTool, RegistryToolHashMap, RuntimeBusEvent, RuntimeBusStream,
    SessionStore, SessionStoreEvent, Store, StoreEvent, StoreEventStream, Tool,
};

#[test]
fn dedicated_modules_and_root_reexports_are_available() {
    fn accepts_tool(_tool: &dyn Tool) {}
    fn accepts_runtime(_runtime: &dyn BrainRuntime) {}
    fn accepts_registry(_registry: &dyn Registry<Item = dyn brain_types::Tool>) {}
    fn accepts_registry_map(_registry: &RegistryHashMap<dyn brain_types::Tool>) {}
    fn accepts_crud_store(
        _store: &dyn CrudStore<
            Key = brain_types::ProjectId,
            Record = brain_types::Project,
            Event = ProjectStoreEvent,
        >,
    ) {
    }
    fn accepts_provider_registry(_registry: &dyn RegistryProvider) {}
    fn accepts_provider_registry_map(_registry: &RegistryProviderHashMap) {}
    fn accepts_loop_registry(_registry: &dyn RegistryLoop) {}
    fn accepts_loop_registry_map(_registry: &RegistryLoopHashMap) {}
    fn accepts_root_store(_store: &dyn Store) {}
    fn accepts_root_store_event(_event: StoreEvent) {}
    fn accepts_root_store_event_stream(_stream: StoreEventStream) {}
    fn accepts_root_project_store(_store: &dyn ProjectStore) {}
    fn accepts_root_project_store_event(_event: ProjectStoreEvent) {}
    fn accepts_root_session_store(_store: &dyn SessionStore) {}
    fn accepts_root_session_store_event(_event: SessionStoreEvent) {}
    fn accepts_root_message_store(_store: &dyn MessageStore) {}
    fn accepts_root_message_store_event(_event: MessageStoreEvent) {}
    fn accepts_root_message_store_key(_key: MessageStoreKey) {}
    fn accepts_root_credential_store(_store: &dyn CredentialStore) {}
    fn accepts_root_credential_store_event(_event: CredentialStoreEvent) {}
    fn accepts_root_credential_store_key(_key: CredentialStoreKey) {}
    fn accepts_root_event_stream(_stream: EventStream) {}
    fn accepts_root_runtime_bus_event(_event: RuntimeBusEvent) {}
    fn accepts_root_runtime_bus_stream(_stream: RuntimeBusStream) {}
    fn accepts_root_agent_loop(_loop_impl: &dyn AgentLoop) {}
    fn accepts_tool_registry(_registry: &dyn RegistryTool) {}
    fn accepts_tool_registry_map(_registry: &RegistryToolHashMap) {}

    fn accepts_module_tool(_tool: &dyn brain_types::tool::Tool) {}
    fn accepts_module_registry(
        _registry: &dyn brain_types::registry::Registry<Item = dyn brain_types::Tool>,
    ) {
    }
    fn accepts_module_crud_store(
        _store: &dyn brain_types::store::CrudStore<
            Key = brain_types::ProjectId,
            Record = brain_types::Project,
            Event = brain_types::store::ProjectStoreEvent,
        >,
    ) {
    }
    fn accepts_module_registry_map(
        _registry: &brain_types::registry::RegistryHashMap<dyn brain_types::Tool>,
    ) {
    }
    fn accepts_module_provider_registry(_registry: &dyn brain_types::registry::RegistryProvider) {}
    fn accepts_module_loop_registry(_registry: &dyn brain_types::registry::RegistryLoop) {}
    fn accepts_module_runtime(_runtime: &dyn brain_types::runtime::BrainRuntime) {}
    fn accepts_module_project_store(_store: &dyn brain_types::store::ProjectStore) {}
    fn accepts_module_project_store_event(_event: brain_types::store::ProjectStoreEvent) {}
    fn accepts_module_session_store(_store: &dyn brain_types::store::SessionStore) {}
    fn accepts_module_session_store_event(_event: brain_types::store::SessionStoreEvent) {}
    fn accepts_module_message_store(_store: &dyn brain_types::store::MessageStore) {}
    fn accepts_module_message_store_event(_event: brain_types::store::MessageStoreEvent) {}
    fn accepts_module_message_store_key(_key: brain_types::store::MessageStoreKey) {}
    fn accepts_module_credential_store(_store: &dyn brain_types::store::CredentialStore) {}
    fn accepts_module_credential_store_event(_event: brain_types::store::CredentialStoreEvent) {}
    fn accepts_module_credential_store_key(_key: brain_types::store::CredentialStoreKey) {}
    fn accepts_module_store(_store: &dyn brain_types::store::Store) {}
    fn accepts_module_store_event(_event: brain_types::store::StoreEvent) {}
    fn accepts_module_store_event_stream(_stream: brain_types::store::StoreEventStream) {}
    fn accepts_module_event_stream(_stream: brain_types::agent_loop::EventStream) {}
    fn accepts_module_runtime_bus_event(_event: brain_types::runtime::RuntimeBusEvent) {}
    fn accepts_module_runtime_bus_stream(_stream: brain_types::runtime::RuntimeBusStream) {}
    fn accepts_module_agent_loop(_loop_impl: &dyn brain_types::agent_loop::AgentLoop) {}
    fn accepts_module_tool_registry(_registry: &dyn brain_types::registry::RegistryTool) {}

    let _ = accepts_tool;
    let _ = accepts_runtime;
    let _ = accepts_registry;
    let _ = accepts_registry_map;
    let _ = accepts_crud_store;
    let _ = accepts_provider_registry;
    let _ = accepts_provider_registry_map;
    let _ = accepts_loop_registry;
    let _ = accepts_loop_registry_map;
    let _ = accepts_root_store;
    let _ = accepts_root_store_event;
    let _ = accepts_root_store_event_stream;
    let _ = accepts_root_project_store;
    let _ = accepts_root_project_store_event;
    let _ = accepts_root_session_store;
    let _ = accepts_root_session_store_event;
    let _ = accepts_root_message_store;
    let _ = accepts_root_message_store_event;
    let _ = accepts_root_message_store_key;
    let _ = accepts_root_credential_store;
    let _ = accepts_root_credential_store_event;
    let _ = accepts_root_credential_store_key;
    let _ = accepts_root_event_stream;
    let _ = accepts_root_runtime_bus_event;
    let _ = accepts_root_runtime_bus_stream;
    let _ = accepts_root_agent_loop;
    let _ = accepts_tool_registry;
    let _ = accepts_tool_registry_map;
    let _ = accepts_module_tool;
    let _ = accepts_module_registry;
    let _ = accepts_module_crud_store;
    let _ = accepts_module_registry_map;
    let _ = accepts_module_provider_registry;
    let _ = accepts_module_loop_registry;
    let _ = accepts_module_runtime;
    let _ = accepts_module_project_store;
    let _ = accepts_module_project_store_event;
    let _ = accepts_module_session_store;
    let _ = accepts_module_session_store_event;
    let _ = accepts_module_message_store;
    let _ = accepts_module_message_store_event;
    let _ = accepts_module_message_store_key;
    let _ = accepts_module_credential_store;
    let _ = accepts_module_credential_store_event;
    let _ = accepts_module_credential_store_key;
    let _ = accepts_module_store;
    let _ = accepts_module_store_event;
    let _ = accepts_module_store_event_stream;
    let _ = accepts_module_event_stream;
    let _ = accepts_module_runtime_bus_event;
    let _ = accepts_module_runtime_bus_stream;
    let _ = accepts_module_agent_loop;
    let _ = accepts_module_tool_registry;
}
