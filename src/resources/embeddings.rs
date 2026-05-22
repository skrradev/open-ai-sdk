use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    core::{HttpClient, Result},
    models::ModelId,
    types::StringOrVec,
};

#[derive(Clone, Debug)]
pub struct EmbeddingsResource {
    client: Arc<HttpClient>,
}

impl EmbeddingsResource {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    pub async fn create(&self, params: EmbeddingCreateParams) -> Result<CreateEmbeddingResponse> {
        self.client.post("/embeddings", params, None).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingCreateParams {
    pub model: ModelId,
    pub input: StringOrVec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl EmbeddingCreateParams {
    pub fn new(model: impl Into<ModelId>, input: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            input: StringOrVec::String(input.into()),
            dimensions: None,
            encoding_format: None,
            user: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn inputs(model: impl Into<ModelId>, input: Vec<String>) -> Self {
        Self {
            model: model.into(),
            input: StringOrVec::Vec(input),
            dimensions: None,
            encoding_format: None,
            user: None,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmbeddingResponse {
    pub object: String,
    pub data: Vec<Embedding>,
    pub model: String,
    pub usage: EmbeddingUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}
