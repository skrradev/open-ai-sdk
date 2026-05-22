//! Rust SDK for the OpenAI API.
//!
//! The crate is organized around a small public client, internal HTTP core,
//! endpoint resources, shared API types, and streaming utilities:
//!
//! - [`OpenAI`] is the public entry point.
//! - [`core`] contains configuration, request options, errors, and HTTP plumbing.
//! - [`resources`] contains endpoint-specific operations and DTOs.
//! - [`types`] contains API shapes reused across resources.
//! - [`streaming`] contains Server-Sent Events helpers.
//! - [`prelude`] contains the most common imports for application code.

pub mod client;
pub mod config;
pub mod core;
pub mod error;
pub mod models;
pub mod prelude;
pub mod request_options;
pub mod resources;
pub mod stream;
pub mod streaming;
pub mod types;

pub use client::OpenAI;
pub use core::{ApiError, Config, Error, RequestOptions, Result};
pub use models::ModelId;
pub use schemars::JsonSchema;
