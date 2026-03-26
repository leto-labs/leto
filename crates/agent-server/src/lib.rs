#![recursion_limit = "512"]

mod compat;
mod http;
mod server;
mod types;
#[cfg(test)]
mod utils;

pub use http::{build_router, serve};
pub use server::AgentServer;
pub use types::{
    AgentServerStatus, CreateProjectRequest, CreateSessionRequest, ErrorBody, ErrorResponse,
    HealthResponse, SessionRuntimeView, TurnRequest,
};
