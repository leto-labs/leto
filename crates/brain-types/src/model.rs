/// Metadata for an AI model, closely mirroring the [models.dev](https://models.dev) schema.
///
/// All fields use `&'static str` / `&'static [...]` so instances can live in
/// `const` preset arrays with zero heap allocation.
///
/// **Deviation from models.dev**: `reasoning` is `Option<&'static [&'static str]>`
/// (listing supported effort levels) rather than a plain `bool`. `None` means the
/// model has no reasoning capability; `Some(&["low", "medium", "high"])` lists the
/// effort levels it accepts, enabling runtime clamping.
#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub family: Option<&'static str>,
    /// `None` = not a reasoning model. `Some(&["low", "medium", "high"])` = supported effort levels.
    pub reasoning: Option<&'static [&'static str]>,
    pub tool_call: bool,
    pub attachment: bool,
    pub structured_output: Option<bool>,
    pub temperature: Option<bool>,
    pub knowledge: Option<&'static str>,
    pub release_date: Option<&'static str>,
    pub last_updated: Option<&'static str>,
    pub open_weights: Option<bool>,
    pub input_modalities: &'static [&'static str],
    pub output_modalities: &'static [&'static str],
    pub cost: Option<ModelCost>,
    pub limit: Option<ModelLimit>,
    /// `"alpha"`, `"beta"`, or `"deprecated"`.
    pub status: Option<&'static str>,
}

/// Per-million-token pricing in USD.
#[derive(Debug, Clone, Copy)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub reasoning: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
    pub input_audio: Option<f64>,
    pub output_audio: Option<f64>,
}

/// Token limits.
#[derive(Debug, Clone, Copy)]
pub struct ModelLimit {
    pub context: u64,
    pub input: Option<u64>,
    pub output: u64,
}
