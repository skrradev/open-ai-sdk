use std::sync::Arc;

use crate::{
    core::{Config, HttpClient, Result},
    resources::{
        chat::ChatResource, embeddings::EmbeddingsResource, files::FilesResource,
        models::ModelsResource, responses::ResponsesResource,
    },
};

#[derive(Clone, Debug)]
pub struct OpenAI {
    http: Arc<HttpClient>,
}

impl OpenAI {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            http: Arc::new(HttpClient::new(config)?),
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(Config::from_env()?)
    }

    pub fn responses(&self) -> ResponsesResource {
        ResponsesResource::new(self.http.clone())
    }

    pub fn chat(&self) -> ChatResource {
        ChatResource::new(self.http.clone())
    }

    pub fn embeddings(&self) -> EmbeddingsResource {
        EmbeddingsResource::new(self.http.clone())
    }

    pub fn files(&self) -> FilesResource {
        FilesResource::new(self.http.clone())
    }

    pub fn models(&self) -> ModelsResource {
        ModelsResource::new(self.http.clone())
    }
}
