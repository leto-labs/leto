use std::path::Path;
use std::sync::Arc;

use agent_runtime::{RuntimeError, ToolExecutionResult};
use provider::ToolDefinition;
use serde_json::Value;
use thiserror::Error;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use crate::registry::{ErasedTool, RegistryToolExecutor, ToolRegistrationError};

wasmtime::component::bindgen!({
    path: "wit",
    world: "tool-plugin",
});

/// Host capability surface made available to tool components.
pub trait ComponentHost: Send + Sync + 'static {
    /// Reads file contents with optional line offset and limit.
    fn read_file(
        &self,
        _path: &str,
        _offset: Option<u64>,
        _limit: Option<u64>,
    ) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-files.read-file".into(),
        ))
    }

    /// Writes content to the target path.
    fn write_file(&self, _path: &str, _content: &str) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-files.write-file".into(),
        ))
    }

    /// Replaces exact text in a file.
    fn edit_file(
        &self,
        _path: &str,
        _old_string: &str,
        _new_string: &str,
    ) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-files.edit-file".into(),
        ))
    }

    /// Applies a patch envelope.
    fn apply_patch(&self, _patch: &str) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-files.apply-patch".into(),
        ))
    }

    /// Lists files in a directory tree.
    fn list_directory(&self, _path: &str, _depth: Option<u32>) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-files.list-directory".into(),
        ))
    }

    /// Performs glob-based file search.
    fn glob_search(&self, _pattern: &str, _path: Option<&str>) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-search.glob-search".into(),
        ))
    }

    /// Performs grep-style content search.
    fn grep(
        &self,
        _pattern: &str,
        _path: Option<&str>,
        _include: Option<&str>,
    ) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-search.grep".into(),
        ))
    }

    /// Runs a shell command.
    fn shell(
        &self,
        _command: &str,
        _working_directory: Option<&str>,
    ) -> Result<String, RuntimeError> {
        Err(RuntimeError::Tool(
            "component host capability not configured: host-process.shell".into(),
        ))
    }
}

/// Errors returned while loading or validating a tool component.
#[derive(Debug, Error)]
pub enum ComponentError {
    /// Failed to configure the Wasmtime engine.
    #[error("failed to configure component engine: {0}")]
    Engine(#[source] wasmtime::Error),
    /// Failed to compile the component.
    #[error("failed to compile component: {0}")]
    Compile(#[source] wasmtime::Error),
    /// Failed to instantiate the component for metadata discovery.
    #[error("failed to instantiate component: {0}")]
    Instantiate(#[source] wasmtime::Error),
    /// Failed to read a component file from disk.
    #[error("failed to read component file {path}: {source}")]
    ReadFile {
        /// Path that failed to load.
        path: String,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// The component returned invalid JSON metadata.
    #[error("component tool `{tool}` returned invalid {field} JSON: {source}")]
    InvalidMetadataJson {
        /// Tool name in the component metadata.
        tool: String,
        /// Metadata field that failed to decode.
        field: &'static str,
        /// Underlying JSON error.
        source: serde_json::Error,
    },
    /// The component exported the same stable tool name more than once.
    #[error("component exported duplicate tool name `{name}`")]
    DuplicateToolName {
        /// Duplicate exported stable tool name.
        name: String,
    },
    /// The component metadata call trapped or returned an invalid value.
    #[error("component metadata discovery failed: {0}")]
    Metadata(#[source] wasmtime::Error),
}

#[derive(Clone)]
struct ComponentStoreState {
    host: Arc<dyn ComponentHost>,
}

impl exports::leto::agent_tools::plugin::Host for ComponentStoreState {}

impl leto::agent_tools::host_files::Host for ComponentStoreState {
    fn read_file(
        &mut self,
        path: String,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<String, String> {
        crate::file_read::component::read_file(self.host.as_ref(), path, offset, limit)
    }

    fn write_file(&mut self, path: String, content: String) -> Result<String, String> {
        crate::file_write::component::write_file(self.host.as_ref(), path, content)
    }

    fn edit_file(
        &mut self,
        path: String,
        old_string: String,
        new_string: String,
    ) -> Result<String, String> {
        crate::file_edit::component::edit_file(self.host.as_ref(), path, old_string, new_string)
    }

    fn apply_patch(&mut self, patch: String) -> Result<String, String> {
        crate::apply_patch::component::apply_patch(self.host.as_ref(), patch)
    }

    fn list_directory(&mut self, path: String, depth: Option<u32>) -> Result<String, String> {
        crate::list_directory::component::list_directory(self.host.as_ref(), path, depth)
    }
}

impl leto::agent_tools::host_search::Host for ComponentStoreState {
    fn glob_search(&mut self, pattern: String, path: Option<String>) -> Result<String, String> {
        crate::glob_search::component::glob_search(self.host.as_ref(), pattern, path)
    }

    fn grep(
        &mut self,
        pattern: String,
        path: Option<String>,
        include: Option<String>,
    ) -> Result<String, String> {
        crate::grep::component::grep(self.host.as_ref(), pattern, path, include)
    }
}

impl leto::agent_tools::host_process::Host for ComponentStoreState {
    fn shell(
        &mut self,
        command: String,
        working_directory: Option<String>,
    ) -> Result<String, String> {
        crate::shell::component::shell(self.host.as_ref(), command, working_directory)
    }
}

#[derive(Clone)]
struct ToolDescriptor {
    definition: ToolDefinition,
}

struct ComponentPluginInner {
    engine: Engine,
    component: Component,
    host: Arc<dyn ComponentHost>,
    tools: Vec<ToolDescriptor>,
}

/// Loaded Wasmtime-backed tool component that can register one or more tools.
#[derive(Clone)]
pub struct ComponentToolPlugin {
    inner: Arc<ComponentPluginInner>,
}

impl ComponentToolPlugin {
    /// Loads a component from raw bytes and discovers its exported tools.
    pub fn from_bytes(
        host: impl ComponentHost,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, ComponentError> {
        let engine = component_engine()?;
        let component = Component::new(&engine, bytes.as_ref()).map_err(ComponentError::Compile)?;
        Self::from_compiled(component, engine, Arc::new(host))
    }

    /// Loads a component from a file path and discovers its exported tools.
    pub fn from_file(
        host: impl ComponentHost,
        path: impl AsRef<Path>,
    ) -> Result<Self, ComponentError> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref).map_err(|source| ComponentError::ReadFile {
            path: path_ref.display().to_string(),
            source,
        })?;
        Self::from_bytes(host, bytes)
    }

    /// Returns the provider-visible tool definitions exported by the component.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.inner
            .tools
            .iter()
            .map(|tool| tool.definition.clone())
            .collect()
    }

    /// Registers all component-exported tools into a registry.
    pub fn register_into(
        &self,
        registry: &mut RegistryToolExecutor,
    ) -> Result<(), ToolRegistrationError> {
        for tool in &self.inner.tools {
            registry.register_erased(Arc::new(ComponentErasedTool {
                plugin: self.clone(),
                definition: tool.definition.clone(),
            }))?;
        }
        Ok(())
    }

    fn from_compiled(
        component: Component,
        engine: Engine,
        host: Arc<dyn ComponentHost>,
    ) -> Result<Self, ComponentError> {
        let tools = discover_tools(&engine, &component, host.clone())?;
        Ok(Self {
            inner: Arc::new(ComponentPluginInner {
                engine,
                component,
                host,
                tools,
            }),
        })
    }

    fn invoke(&self, tool_name: &str, input: Value) -> Result<ToolExecutionResult, RuntimeError> {
        let linker = component_linker(&self.inner.engine)?;
        let mut store = Store::new(
            &self.inner.engine,
            ComponentStoreState {
                host: self.inner.host.clone(),
            },
        );
        let bindings = ToolPlugin::instantiate(&mut store, &self.inner.component, &linker)
            .map_err(|error| {
                RuntimeError::Tool(format!(
                    "failed to instantiate component for tool `{tool_name}`: {error}"
                ))
            })?;
        let input_json = serde_json::to_string(&input).map_err(|error| {
            RuntimeError::Tool(format!(
                "failed to encode component input for `{tool_name}`: {error}"
            ))
        })?;
        let output = bindings
            .plugin()
            .call_invoke_tool(&mut store, tool_name, &input_json)
            .map_err(|error| {
                RuntimeError::Tool(format!(
                    "component execution failed for tool `{tool_name}`: {error}"
                ))
            })?
            .map_err(|message| RuntimeError::Tool(format!("{tool_name}: {message}")))?;
        let output = serde_json::from_str::<Value>(&output).map_err(|error| {
            RuntimeError::Tool(format!(
                "component tool `{tool_name}` returned invalid JSON: {error}"
            ))
        })?;
        Ok(ToolExecutionResult::success(output))
    }
}

struct ComponentErasedTool {
    plugin: ComponentToolPlugin,
    definition: ToolDefinition,
}

impl ErasedTool for ComponentErasedTool {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn execute<'a>(
        &'a self,
        input: Value,
    ) -> futures::future::BoxFuture<'a, Result<ToolExecutionResult, RuntimeError>> {
        Box::pin(async move { self.plugin.invoke(&self.definition.name, input) })
    }
}

fn component_engine() -> Result<Engine, ComponentError> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).map_err(ComponentError::Engine)
}

fn component_linker(engine: &Engine) -> Result<Linker<ComponentStoreState>, RuntimeError> {
    let mut linker = Linker::new(engine);
    ToolPlugin::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)
        .map_err(|error| {
            RuntimeError::Tool(format!("failed to link component imports: {error}"))
        })?;
    Ok(linker)
}

fn discover_tools(
    engine: &Engine,
    component: &Component,
    host: Arc<dyn ComponentHost>,
) -> Result<Vec<ToolDescriptor>, ComponentError> {
    let linker = component_linker(engine)
        .map_err(|error| ComponentError::Instantiate(wasmtime::Error::msg(error.to_string())))?;
    let mut store = Store::new(engine, ComponentStoreState { host });
    let bindings = ToolPlugin::instantiate(&mut store, component, &linker)
        .map_err(ComponentError::Instantiate)?;
    let metadata = bindings
        .plugin()
        .call_list_tools(&mut store)
        .map_err(ComponentError::Metadata)?;
    let mut names = std::collections::BTreeSet::new();
    let mut tools = Vec::with_capacity(metadata.len());
    for tool in metadata {
        if !names.insert(tool.name.clone()) {
            return Err(ComponentError::DuplicateToolName { name: tool.name });
        }
        let input_schema = serde_json::from_str(&tool.input_schema_json).map_err(|source| {
            ComponentError::InvalidMetadataJson {
                tool: tool.name.clone(),
                field: "input_schema_json",
                source,
            }
        })?;
        let mut definition =
            ToolDefinition::new(tool.name.clone(), tool.description.clone(), input_schema);
        if let Some(output_schema_json) = tool.output_schema_json.as_ref() {
            let output_schema = serde_json::from_str(output_schema_json).map_err(|source| {
                ComponentError::InvalidMetadataJson {
                    tool: tool.name.clone(),
                    field: "output_schema_json",
                    source,
                }
            })?;
            definition = definition.with_output_schema(output_schema);
        }
        tools.push(ToolDescriptor { definition });
    }
    Ok(tools)
}
