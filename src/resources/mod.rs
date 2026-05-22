//! Endpoint resources.
//!
//! Each module owns one public OpenAI API resource surface. Resource structs
//! expose async operations, while request and response DTOs live beside the
//! operations they belong to.

pub mod chat;
pub mod common;
pub mod embeddings;
pub mod files;
pub mod models;
pub mod responses;
