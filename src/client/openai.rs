use std::sync::Arc;

use crate::{
    core::{Config, HttpClient, Result},
    resources::{
        chat::ChatResource, embeddings::EmbeddingsResource, files::FilesResource,
        models::ModelsResource, responses::ResponsesResource,
    },
};

#[derive(Clone, Debug)]
/// Public OpenAI API client.
///
/// Create a client with [`OpenAI::from_env`] to read `OPENAI_API_KEY`, or pass
/// an explicit [`Config`] to [`OpenAI::new`].
///
/// # Example
///
/// ```no_run
/// use open_ai_sdk::{OpenAI, resources::chat::{ChatCompletionCreateParams, ChatMessage}};
///
/// # async fn run() -> open_ai_sdk::Result<()> {
/// let client = OpenAI::from_env()?;
/// let completion = client
///     .chat()
///     .completions()
///     .create(
///         ChatCompletionCreateParams::new("gpt-4o")
///             .message(ChatMessage::user("Say hello.")),
///     )
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct OpenAI {
    http: Arc<HttpClient>,
}

impl OpenAI {
    /// Creates a client from an explicit configuration.
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            http: Arc::new(HttpClient::new(config)?),
        })
    }

    /// Creates a client from environment variables.
    ///
    /// Reads:
    ///
    /// - `OPENAI_API_KEY`
    /// - `OPENAI_ORG_ID`, optional
    /// - `OPENAI_PROJECT_ID`, optional
    /// - `OPENAI_BASE_URL`, optional
    pub fn from_env() -> Result<Self> {
        Self::new(Config::from_env()?)
    }

    /// Accesses the Responses API resource.
    pub fn responses(&self) -> ResponsesResource {
        ResponsesResource::new(self.http.clone())
    }

    /// Accesses Chat resources, including Chat Completions.
    pub fn chat(&self) -> ChatResource {
        ChatResource::new(self.http.clone())
    }

    /// Accesses the Embeddings API resource.
    pub fn embeddings(&self) -> EmbeddingsResource {
        EmbeddingsResource::new(self.http.clone())
    }

    /// Accesses the Files API resource.
    pub fn files(&self) -> FilesResource {
        FilesResource::new(self.http.clone())
    }

    /// Accesses the Models API resource.
    pub fn models(&self) -> ModelsResource {
        ModelsResource::new(self.http.clone())
    }
}
