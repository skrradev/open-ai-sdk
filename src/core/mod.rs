//! Internal SDK core.
//!
//! Public applications usually import these through the crate root or
//! [`crate::prelude`]. Resource modules use [`HttpClient`] internally so endpoint
//! code stays focused on paths, params, and return types.

mod config;
mod error;
mod http;
mod request_options;

pub use config::Config;
pub(crate) use error::ApiErrorEnvelope;
pub use error::{ApiError, Error, Result};
pub(crate) use http::HttpClient;
pub use request_options::RequestOptions;
