use std::path::Path;

use agent_client_protocol as acp;
use brain_core::{Brain, Project, ProjectConfig, normalize_project_root};

use super::errors::map_brain_error;

pub async fn resolve_project(brain: &Brain, cwd: &Path) -> Result<Project, acp::Error> {
    let normalized_root = normalize_project_root(cwd);
    if let Some(project) = brain
        .store
        .project_find_by_root(&normalized_root)
        .await
        .map_err(map_brain_error)?
    {
        return Ok(project);
    }

    let project_name = normalized_root
        .file_name()
        .and_then(|segment| segment.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "brain".to_owned());
    let config = resolve_config(&normalized_root);
    let project = Project::new(Some(project_name), Some(normalized_root), config);
    brain
        .store
        .project_create(project)
        .await
        .map_err(map_brain_error)
}

fn resolve_config(cwd: &Path) -> ProjectConfig {
    let mut config = brain_config::resolve_fs_config(cwd).unwrap_or_else(|error| {
        tracing::warn!("config resolution failed, using defaults: {error}");
        ProjectConfig::default()
    });

    if config.agent.system_prompt.is_none()
        && let Ok(Some(agents_md)) = brain_config::load_root_agents_md(cwd)
    {
        config.agent.system_prompt = Some(agents_md);
    }

    config
}
