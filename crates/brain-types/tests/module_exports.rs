use brain_types::{AgentLoop, EventStream, ProjectStore, Store, Tool};

#[test]
fn dedicated_modules_and_root_reexports_are_available() {
    fn accepts_tool(_tool: &dyn Tool) {}
    fn accepts_root_store(_store: &dyn Store) {}
    fn accepts_root_project_store(_store: &dyn ProjectStore) {}
    fn accepts_root_event_stream(_stream: EventStream) {}
    fn accepts_root_agent_loop(_loop_impl: &dyn AgentLoop) {}

    fn accepts_module_tool(_tool: &dyn brain_types::tool::Tool) {}
    fn accepts_module_project_store(_store: &dyn brain_types::store::ProjectStore) {}
    fn accepts_module_store(_store: &dyn brain_types::store::Store) {}
    fn accepts_module_event_stream(_stream: brain_types::agent_loop::EventStream) {}
    fn accepts_module_agent_loop(_loop_impl: &dyn brain_types::agent_loop::AgentLoop) {}

    let _ = accepts_tool;
    let _ = accepts_root_store;
    let _ = accepts_root_project_store;
    let _ = accepts_root_event_stream;
    let _ = accepts_root_agent_loop;
    let _ = accepts_module_tool;
    let _ = accepts_module_project_store;
    let _ = accepts_module_store;
    let _ = accepts_module_event_stream;
    let _ = accepts_module_agent_loop;
}
