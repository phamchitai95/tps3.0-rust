//! Shared types for TPS 3.0 (Rust rewrite of TC59 / tps59).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Row shape matching the Dart/Node 1.0 API: string keys, JSON-friendly values.
pub type Row = HashMap<String, Value>;
pub type Args = HashMap<String, Value>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("Không có chức năng \"{0}\".")]
    UnknownCommand(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self::Message(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        Self::Message(s.to_string())
    }
}

/// Logged-in user context (mirrors Dart `Ctx`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ctx {
    pub user: Option<Row>,
}

/// Role numbers — same as tps59 / Dart core.
pub mod role {
    pub const ADMIN: i32 = 1;
    pub const NHAN_VIEN: i32 = 3;
    pub const CTV: i32 = 4;
}
