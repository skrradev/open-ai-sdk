use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    core::{HttpClient, RequestOptions, Result},
    models::ModelId,
    streaming::SseStream,
};

#[derive(Clone, Debug)]
pub struct ResponsesResource {
    client: Arc<HttpClient>,
}

impl ResponsesResource {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    pub async fn create(&self, params: ResponseCreateParams) -> Result<Response> {
        self.create_with_options(params, None).await
    }

    pub async fn create_with_options(
        &self,
        params: ResponseCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<Response> {
        self.client.post("/responses", params, options).await
    }

    pub async fn stream(&self, mut params: ResponseCreateParams) -> Result<SseStream> {
        params.stream = Some(true);
        self.stream_with_options(params, None).await
    }

    pub async fn stream_with_options(
        &self,
        mut params: ResponseCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<SseStream> {
        params.stream = Some(true);
        self.client.post_stream("/responses", params, options).await
    }

    pub async fn retrieve(&self, response_id: impl AsRef<str>) -> Result<Response> {
        self.client
            .get(&format!("/responses/{}", response_id.as_ref()), None)
            .await
    }

    pub async fn delete(&self, response_id: impl AsRef<str>) -> Result<ResponseDeleted> {
        self.client
            .delete(&format!("/responses/{}", response_id.as_ref()), None)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCreateParams {
    pub model: ModelId,
    pub input: ResponseInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl ResponseCreateParams {
    pub fn new(model: impl Into<ModelId>, input: impl Into<ResponseInput>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            previous_response_id: None,
            stream: None,
            temperature: None,
            top_p: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn max_output_tokens(mut self, max_output_tokens: u32) -> Self {
        self.max_output_tokens = Some(max_output_tokens);
        self
    }

    pub fn extra(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseInput {
    Text(String),
    Items(Vec<ResponseInputItem>),
}

impl From<&str> for ResponseInput {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for ResponseInput {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec<ResponseInputItem>> for ResponseInput {
    fn from(value: Vec<ResponseInputItem>) -> Self {
        Self::Items(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseInputItem {
    pub role: String,
    pub content: Vec<ResponseInputContent>,
}

impl ResponseInputItem {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![ResponseInputContent::input_text(text)],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputContent {
    InputText { text: String },
    InputImage { image_url: String },
}

impl ResponseInputContent {
    pub fn input_text(text: impl Into<String>) -> Self {
        Self::InputText { text: text.into() }
    }

    pub fn input_image(image_url: impl Into<String>) -> Self {
        Self::InputImage {
            image_url: image_url.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created_at: Option<u64>,
    pub model: Option<String>,
    pub status: Option<String>,
    pub output: Option<Vec<ResponseOutputItem>>,
    pub output_text: Option<String>,
    pub usage: Option<ResponseUsage>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOutputItem {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    pub role: Option<String>,
    pub content: Option<Vec<ResponseOutputContent>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOutputContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDeleted {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
