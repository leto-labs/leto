mod messages {
    use super::parts::PartDoc;
    use super::prelude::*;

    include!("messages.rs");
}
mod parts {
    use super::messages::{MessageModelDoc, MessageTimeCreatedDoc};
    use super::prelude::*;

    include!("parts.rs");
}
mod paths {
    use super::prelude::*;

    include!("paths.rs");
}
mod queries {
    use super::prelude::*;

    include!("queries.rs");
}
mod requests {
    use super::messages::{MessageModelDoc, OutputFormatDoc, PermissionResponseKindDoc};
    use super::parts::{
        AgentPartInputDoc, FilePartInputDoc, FilePartKindDoc, FilePartSourceDoc,
        SubtaskPartInputDoc, TextPartInputDoc,
    };
    use super::prelude::*;

    include!("requests.rs");
}
mod status {
    use super::prelude::*;

    include!("status.rs");
}

mod prelude {
    pub(super) use schemars::JsonSchema;
    pub(super) use serde::{Deserialize, Serialize};
    pub(super) use serde_json::Value;

    pub(super) use super::super::errors::{
        ApiErrorDoc, ContextOverflowErrorDoc, MessageAbortedErrorDoc, MessageOutputLengthErrorDoc,
        ProviderAuthErrorDoc, StructuredOutputErrorDoc, UnknownErrorDoc,
    };
    pub(super) use super::super::files::{
        FileDiffDoc, FilePartSourceTextDoc, FileSourceDoc, ResourceSourceDoc, SymbolSourceDoc,
    };
    pub(super) use super::super::permission::PermissionRuleset;
    pub(super) use super::super::project::ProjectSummaryDoc;
    pub(super) use super::super::provider::MessageTokensCacheDoc;
}

pub use messages::*;
pub use parts::*;
pub use paths::*;
pub use queries::*;
pub use requests::*;
pub use status::*;

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
