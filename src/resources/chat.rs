use std::{collections::BTreeMap, sync::Arc};

use schemars::{schema_for, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    core::{Error, HttpClient, RequestOptions, Result},
    models::ModelId,
    streaming::SseStream,
};

#[derive(Clone, Debug)]
/// Chat namespace.
///
/// Use [`ChatResource::completions`] to access Chat Completions.
pub struct ChatResource {
    client: Arc<HttpClient>,
}

impl ChatResource {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Accesses Chat Completions operations.
    pub fn completions(&self) -> ChatCompletionsResource {
        ChatCompletionsResource::new(self.client.clone())
    }
}

#[derive(Clone, Debug)]
/// Chat Completions operations.
pub struct ChatCompletionsResource {
    client: Arc<HttpClient>,
}

impl ChatCompletionsResource {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    /// Creates a Chat Completion.
    pub async fn create(&self, params: ChatCompletionCreateParams) -> Result<ChatCompletion> {
        self.create_with_options(params, None).await
    }

    /// Creates a Chat Completion with per-request options.
    pub async fn create_with_options(
        &self,
        params: ChatCompletionCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<ChatCompletion> {
        self.client.post("/chat/completions", params, options).await
    }

    /// Creates a Chat Completion and parses the first assistant message as `T`.
    ///
    /// This is the ergonomic structured-output helper. It derives a JSON Schema
    /// from `T`, sets `response_format` automatically, sends the request, then
    /// deserializes the first assistant text message into `T`.
    ///
    /// # Type Parameters
    ///
    /// `T` must implement [`serde::de::DeserializeOwned`] and
    /// [`JsonSchema`]. Derive both with `serde` and `schemars`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use open_ai_sdk::{
    ///     OpenAI, JsonSchema,
    ///     resources::chat::{ChatCompletionCreateParams, ChatMessage},
    /// };
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    /// #[schemars(deny_unknown_fields)]
    /// struct Event {
    ///     city: String,
    ///     date: String,
    /// }
    ///
    /// # async fn run() -> open_ai_sdk::Result<()> {
    /// let client = OpenAI::from_env()?;
    /// let parsed = client.chat().completions().parse::<Event>(
    ///     ChatCompletionCreateParams::new("gpt-4o")
    ///         .message(ChatMessage::user("Extract city and date: Paris, June 3.")),
    ///     "event",
    /// ).await?;
    ///
    /// println!("{:?}", parsed.parsed);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn parse<T>(
        &self,
        params: ChatCompletionCreateParams,
        schema_name: impl Into<String>,
    ) -> Result<ParsedChatCompletion<T>>
    where
        T: DeserializeOwned + JsonSchema,
    {
        self.parse_with_options(params, schema_name, None).await
    }

    /// Same as [`ChatCompletionsResource::parse`], with per-request options.
    pub async fn parse_with_options<T>(
        &self,
        params: ChatCompletionCreateParams,
        schema_name: impl Into<String>,
        options: Option<RequestOptions>,
    ) -> Result<ParsedChatCompletion<T>>
    where
        T: DeserializeOwned + JsonSchema,
    {
        let params = params.json_schema_for::<T>(schema_name);
        let completion = self.create_with_options(params, options).await?;
        let parsed = parse_chat_completion_content(&completion)?;
        Ok(ParsedChatCompletion { completion, parsed })
    }

    /// Creates a raw Server-Sent Events stream.
    ///
    /// This returns raw SSE events. Use [`ChatCompletionsResource::stream_typed`]
    /// for parsed Chat Completion chunks.
    pub async fn stream(&self, mut params: ChatCompletionCreateParams) -> Result<SseStream> {
        params.stream = Some(true);
        self.client
            .post_stream("/chat/completions", params, None)
            .await
    }

    /// Creates a typed Chat Completions stream.
    ///
    /// Automatically sets `stream: true` and yields [`ChatCompletionChunk`]
    /// values through [`ChatCompletionStream::next_chunk`].
    pub async fn stream_typed(
        &self,
        mut params: ChatCompletionCreateParams,
    ) -> Result<ChatCompletionStream> {
        params.stream = Some(true);
        self.client
            .post_stream("/chat/completions", params, None)
            .await
            .map(ChatCompletionStream::new)
    }
}

/// Typed Chat Completions stream.
///
/// Call [`ChatCompletionStream::next_chunk`] until it returns `Ok(None)`.
pub struct ChatCompletionStream {
    inner: SseStream,
}

#[derive(Debug, Clone)]
/// Result returned by [`ChatCompletionsResource::parse`].
pub struct ParsedChatCompletion<T> {
    /// Raw Chat Completion response.
    pub completion: ChatCompletion,
    /// Parsed value deserialized from the first assistant text message.
    pub parsed: T,
}

fn parse_chat_completion_content<T>(completion: &ChatCompletion) -> Result<T>
where
    T: DeserializeOwned,
{
    let choice = completion
        .choices
        .first()
        .ok_or_else(|| Error::Parse("completion contained no choices".to_string()))?;
    match &choice.message.content {
        ChatMessageContent::Text(content) => serde_json::from_str(content).map_err(Error::Json),
        ChatMessageContent::Parts(_) => Err(Error::Parse(
            "first completion choice did not contain text content".to_string(),
        )),
    }
}

impl ChatCompletionStream {
    pub(crate) fn new(inner: SseStream) -> Self {
        Self { inner }
    }

    /// Returns the next typed chunk, or `Ok(None)` after `[DONE]`.
    pub async fn next_chunk(&mut self) -> Result<Option<ChatCompletionChunk>> {
        while let Some(event) = self.inner.next_json::<serde_json::Value>().await? {
            if event == serde_json::Value::String("[DONE]".to_string()) {
                return Ok(None);
            }
            return serde_json::from_value(event).map(Some).map_err(Error::Json);
        }
        Ok(None)
    }

    /// Converts this typed stream back into the raw SSE stream.
    pub fn into_sse_stream(self) -> SseStream {
        self.inner
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Parameters for creating a Chat Completion.
///
/// The type is endpoint-first and model-flexible. It includes typed fields for
/// common official SDK parameters and an [`extra`](Self::extra) map for forward
/// compatibility with new API parameters.
pub struct ChatCompletionCreateParams {
    pub model: ModelId,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatAudioParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<ChatFunctionCallOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<ChatFunctionDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<BTreeMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<ChatModality>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<ChatPredictionContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<PromptCacheRetention>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ChatReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ChatResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ChatServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ChatStreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ChatToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<ChatVerbosity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_options: Option<ChatWebSearchOptions>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl ChatCompletionCreateParams {
    /// Creates params for the given model.
    pub fn new(model: impl Into<ModelId>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            audio: None,
            frequency_penalty: None,
            function_call: None,
            functions: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            metadata: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            prediction: None,
            presence_penalty: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning_effort: None,
            response_format: None,
            safety_identifier: None,
            seed: None,
            service_tier: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            temperature: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            verbosity: None,
            web_search_options: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn max_completion_tokens(mut self, max_completion_tokens: u32) -> Self {
        self.max_completion_tokens = Some(max_completion_tokens);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn frequency_penalty(mut self, frequency_penalty: f64) -> Self {
        self.frequency_penalty = Some(frequency_penalty);
        self
    }

    pub fn presence_penalty(mut self, presence_penalty: f64) -> Self {
        self.presence_penalty = Some(presence_penalty);
        self
    }

    pub fn metadata(mut self, metadata: BTreeMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn metadata_pair(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata
            .get_or_insert_with(BTreeMap::new)
            .insert(key.into(), value.into());
        self
    }

    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    pub fn logprobs(mut self, logprobs: bool) -> Self {
        self.logprobs = Some(logprobs);
        self
    }

    pub fn top_logprobs(mut self, top_logprobs: u8) -> Self {
        self.top_logprobs = Some(top_logprobs);
        self
    }

    pub fn logit_bias(mut self, logit_bias: BTreeMap<String, f64>) -> Self {
        self.logit_bias = Some(logit_bias);
        self
    }

    pub fn logit_bias_token(mut self, token: impl Into<String>, bias: f64) -> Self {
        self.logit_bias
            .get_or_insert_with(BTreeMap::new)
            .insert(token.into(), bias);
        self
    }

    pub fn response_format(mut self, response_format: ChatResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    pub fn reasoning_effort(mut self, reasoning_effort: ChatReasoningEffort) -> Self {
        self.reasoning_effort = Some(reasoning_effort);
        self
    }

    pub fn verbosity(mut self, verbosity: ChatVerbosity) -> Self {
        self.verbosity = Some(verbosity);
        self
    }

    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn stop(mut self, stop: impl Into<StopSequences>) -> Self {
        self.stop = Some(stop.into());
        self
    }

    pub fn store(mut self, store: bool) -> Self {
        self.store = Some(store);
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn safety_identifier(mut self, safety_identifier: impl Into<String>) -> Self {
        self.safety_identifier = Some(safety_identifier.into());
        self
    }

    pub fn prompt_cache_key(mut self, prompt_cache_key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(prompt_cache_key.into());
        self
    }

    pub fn prompt_cache_retention(mut self, retention: PromptCacheRetention) -> Self {
        self.prompt_cache_retention = Some(retention);
        self
    }

    pub fn service_tier(mut self, service_tier: ChatServiceTier) -> Self {
        self.service_tier = Some(service_tier);
        self
    }

    pub fn modality(mut self, modality: ChatModality) -> Self {
        self.modalities.get_or_insert_with(Vec::new).push(modality);
        self
    }

    pub fn modalities(mut self, modalities: Vec<ChatModality>) -> Self {
        self.modalities = Some(modalities);
        self
    }

    pub fn audio(mut self, audio: ChatAudioParam) -> Self {
        self.audio = Some(audio);
        self
    }

    pub fn prediction(mut self, prediction: ChatPredictionContent) -> Self {
        self.prediction = Some(prediction);
        self
    }

    pub fn parallel_tool_calls(mut self, parallel_tool_calls: bool) -> Self {
        self.parallel_tool_calls = Some(parallel_tool_calls);
        self
    }

    pub fn tool(mut self, tool: ChatTool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    pub fn tools(mut self, tools: Vec<ChatTool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn tool_choice(mut self, tool_choice: ChatToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    pub fn function(mut self, function: ChatFunctionDefinition) -> Self {
        self.functions.get_or_insert_with(Vec::new).push(function);
        self
    }

    pub fn function_call(mut self, function_call: ChatFunctionCallOption) -> Self {
        self.function_call = Some(function_call);
        self
    }

    pub fn stream_options(mut self, stream_options: ChatStreamOptions) -> Self {
        self.stream_options = Some(stream_options);
        self
    }

    pub fn web_search_options(mut self, web_search_options: ChatWebSearchOptions) -> Self {
        self.web_search_options = Some(web_search_options);
        self
    }

    pub fn extra(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    pub fn json_schema(
        mut self,
        name: impl Into<String>,
        schema: impl Into<serde_json::Value>,
    ) -> Self {
        self.response_format = Some(ChatResponseFormat::json_schema(name, schema));
        self
    }

    pub fn json_schema_for<T>(mut self, name: impl Into<String>) -> Self
    where
        T: JsonSchema,
    {
        self.response_format = Some(ChatResponseFormat::json_schema_for::<T>(name));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatVerbosity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatModality {
    Text,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCacheRetention {
    InMemory,
    #[serde(rename = "24h")]
    TwentyFourHours,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopSequences {
    One(String),
    Many(Vec<String>),
}

impl From<&str> for StopSequences {
    fn from(value: &str) -> Self {
        Self::One(value.to_string())
    }
}

impl From<String> for StopSequences {
    fn from(value: String) -> Self {
        Self::One(value)
    }
}

impl From<Vec<String>> for StopSequences {
    fn from(value: Vec<String>) -> Self {
        Self::Many(value)
    }
}

impl From<Vec<&str>> for StopSequences {
    fn from(value: Vec<&str>) -> Self {
        Self::Many(value.into_iter().map(str::to_string).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

impl ChatStreamOptions {
    pub fn new() -> Self {
        Self {
            include_obfuscation: None,
            include_usage: None,
        }
    }

    pub fn include_obfuscation(mut self, include_obfuscation: bool) -> Self {
        self.include_obfuscation = Some(include_obfuscation);
        self
    }

    pub fn include_usage(mut self, include_usage: bool) -> Self {
        self.include_usage = Some(include_usage);
        self
    }
}

impl Default for ChatStreamOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAudioParam {
    pub format: ChatAudioFormat,
    pub voice: ChatVoice,
}

impl ChatAudioParam {
    pub fn new(format: ChatAudioFormat, voice: impl Into<ChatVoice>) -> Self {
        Self {
            format,
            voice: voice.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatAudioFormat {
    Wav,
    Aac,
    Mp3,
    Flac,
    Opus,
    Pcm16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatVoice {
    Name(String),
    Id { id: String },
}

impl From<&str> for ChatVoice {
    fn from(value: &str) -> Self {
        Self::Name(value.to_string())
    }
}

impl From<String> for ChatVoice {
    fn from(value: String) -> Self {
        Self::Name(value)
    }
}

impl ChatVoice {
    pub fn id(id: impl Into<String>) -> Self {
        Self::Id { id: id.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPredictionContent {
    pub content: ChatPredictionContentValue,
    #[serde(rename = "type")]
    pub prediction_type: ChatPredictionType,
}

impl ChatPredictionContent {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: ChatPredictionContentValue::Text(content.into()),
            prediction_type: ChatPredictionType::Content,
        }
    }

    pub fn parts(content: Vec<ChatContentPart>) -> Self {
        Self {
            content: ChatPredictionContentValue::Parts(content),
            prediction_type: ChatPredictionType::Content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatPredictionContentValue {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatPredictionType {
    Content,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatResponseFormat {
    Text,
    JsonObject,
    JsonSchema {
        json_schema: ChatResponseFormatJsonSchema,
    },
}

impl ChatResponseFormat {
    pub fn text() -> Self {
        Self::Text
    }

    pub fn json_object() -> Self {
        Self::JsonObject
    }

    pub fn json_schema(name: impl Into<String>, schema: impl Into<serde_json::Value>) -> Self {
        Self::JsonSchema {
            json_schema: ChatResponseFormatJsonSchema::new(name, schema),
        }
    }

    pub fn json_schema_for<T>(name: impl Into<String>) -> Self
    where
        T: JsonSchema,
    {
        Self::JsonSchema {
            json_schema: ChatResponseFormatJsonSchema::for_type::<T>(name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponseFormatJsonSchema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl ChatResponseFormatJsonSchema {
    pub fn new(name: impl Into<String>, schema: impl Into<serde_json::Value>) -> Self {
        Self {
            name: name.into(),
            description: None,
            schema: schema.into(),
            strict: None,
        }
    }

    pub fn for_type<T>(name: impl Into<String>) -> Self
    where
        T: JsonSchema,
    {
        let schema = schema_for!(T);
        Self::new(
            name,
            serde_json::to_value(schema).expect("schema serialization cannot fail"),
        )
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    pub fn developer(content: impl Into<String>) -> Self {
        Self::new("developer", content)
    }

    fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: ChatMessageContent::Text(content.into()),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatMessageContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatContentPart {
    Text { text: String },
    ImageUrl { image_url: ChatImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatTool {
    Function { function: ChatFunctionDefinition },
    Custom { custom: ChatCustomTool },
}

impl ChatTool {
    pub fn function(function: ChatFunctionDefinition) -> Self {
        Self::Function { function }
    }

    pub fn custom(custom: ChatCustomTool) -> Self {
        Self::Custom { custom }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl ChatFunctionDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters: None,
            strict: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn parameters(mut self, parameters: impl Into<serde_json::Value>) -> Self {
        self.parameters = Some(parameters.into());
        self
    }

    pub fn parameters_for<T>(mut self) -> Self
    where
        T: JsonSchema,
    {
        let schema = schema_for!(T);
        self.parameters =
            Some(serde_json::to_value(schema).expect("schema serialization cannot fail"));
        self
    }

    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCustomTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ChatCustomToolFormat>,
}

impl ChatCustomTool {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            format: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn format(mut self, format: ChatCustomToolFormat) -> Self {
        self.format = Some(format);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCustomToolFormat {
    Text,
    Grammar { grammar: ChatGrammar },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGrammar {
    pub definition: String,
    pub syntax: ChatGrammarSyntax,
}

impl ChatGrammar {
    pub fn lark(definition: impl Into<String>) -> Self {
        Self {
            definition: definition.into(),
            syntax: ChatGrammarSyntax::Lark,
        }
    }

    pub fn regex(definition: impl Into<String>) -> Self {
        Self {
            definition: definition.into(),
            syntax: ChatGrammarSyntax::Regex,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatGrammarSyntax {
    Lark,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatToolChoice {
    Mode(ChatToolChoiceMode),
    Function(ChatNamedFunctionToolChoice),
    Custom(ChatNamedCustomToolChoice),
    Allowed(ChatAllowedToolChoice),
}

impl ChatToolChoice {
    pub fn none() -> Self {
        Self::Mode(ChatToolChoiceMode::None)
    }

    pub fn auto() -> Self {
        Self::Mode(ChatToolChoiceMode::Auto)
    }

    pub fn required() -> Self {
        Self::Mode(ChatToolChoiceMode::Required)
    }

    pub fn function(name: impl Into<String>) -> Self {
        Self::Function(ChatNamedFunctionToolChoice::new(name))
    }

    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(ChatNamedCustomToolChoice::new(name))
    }

    pub fn allowed(allowed_tools: ChatAllowedTools) -> Self {
        Self::Allowed(ChatAllowedToolChoice {
            allowed_tools,
            choice_type: ChatAllowedToolChoiceType::AllowedTools,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatToolChoiceMode {
    None,
    Auto,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatNamedFunctionToolChoice {
    pub function: ChatNamedFunction,
    #[serde(rename = "type")]
    pub choice_type: ChatNamedFunctionToolChoiceType,
}

impl ChatNamedFunctionToolChoice {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            function: ChatNamedFunction { name: name.into() },
            choice_type: ChatNamedFunctionToolChoiceType::Function,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatNamedFunction {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatNamedFunctionToolChoiceType {
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatNamedCustomToolChoice {
    pub custom: ChatNamedCustom,
    #[serde(rename = "type")]
    pub choice_type: ChatNamedCustomToolChoiceType,
}

impl ChatNamedCustomToolChoice {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            custom: ChatNamedCustom { name: name.into() },
            choice_type: ChatNamedCustomToolChoiceType::Custom,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatNamedCustom {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatNamedCustomToolChoiceType {
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAllowedToolChoice {
    pub allowed_tools: ChatAllowedTools,
    #[serde(rename = "type")]
    pub choice_type: ChatAllowedToolChoiceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAllowedTools {
    pub mode: ChatAllowedToolsMode,
    pub tools: Vec<serde_json::Value>,
}

impl ChatAllowedTools {
    pub fn new(mode: ChatAllowedToolsMode, tools: Vec<serde_json::Value>) -> Self {
        Self { mode, tools }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatAllowedToolsMode {
    Auto,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatAllowedToolChoiceType {
    AllowedTools,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatFunctionCallOption {
    Mode(ChatFunctionCallMode),
    Named(ChatFunctionCallName),
}

impl ChatFunctionCallOption {
    pub fn none() -> Self {
        Self::Mode(ChatFunctionCallMode::None)
    }

    pub fn auto() -> Self {
        Self::Mode(ChatFunctionCallMode::Auto)
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(ChatFunctionCallName { name: name.into() })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatFunctionCallMode {
    None,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunctionCallName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatWebSearchOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<ChatSearchContextSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<ChatWebSearchUserLocation>,
}

impl ChatWebSearchOptions {
    pub fn new() -> Self {
        Self {
            search_context_size: None,
            user_location: None,
        }
    }

    pub fn search_context_size(mut self, size: ChatSearchContextSize) -> Self {
        self.search_context_size = Some(size);
        self
    }

    pub fn user_location(mut self, location: ChatWebSearchUserLocation) -> Self {
        self.user_location = Some(location);
        self
    }
}

impl Default for ChatWebSearchOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatSearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatWebSearchUserLocation {
    pub approximate: ChatApproximateLocation,
    #[serde(rename = "type")]
    pub location_type: ChatWebSearchUserLocationType,
}

impl ChatWebSearchUserLocation {
    pub fn approximate(approximate: ChatApproximateLocation) -> Self {
        Self {
            approximate,
            location_type: ChatWebSearchUserLocationType::Approximate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatWebSearchUserLocationType {
    Approximate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatApproximateLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

impl ChatApproximateLocation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn city(mut self, city: impl Into<String>) -> Self {
        self.city = Some(city.into());
        self
    }

    pub fn country(mut self, country: impl Into<String>) -> Self {
        self.country = Some(country.into());
        self
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletion {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<ChatUsage>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChunkChoice>,
    pub usage: Option<ChatUsage>,
    pub service_tier: Option<String>,
    pub system_fingerprint: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunkChoice {
    pub index: u32,
    pub delta: ChatCompletionDelta,
    pub finish_reason: Option<String>,
    pub logprobs: Option<ChatChoiceLogprobs>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub refusal: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionDeltaToolCall>>,
    pub function_call: Option<ChatCompletionDeltaFunctionCall>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionDeltaToolCall {
    pub index: u32,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub tool_type: Option<String>,
    pub function: Option<ChatCompletionDeltaToolFunction>,
    pub custom: Option<ChatCompletionDeltaCustomTool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionDeltaToolFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionDeltaCustomTool {
    pub name: Option<String>,
    pub input: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionDeltaFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<ChatChoiceLogprobs>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoiceLogprobs {
    pub content: Option<Vec<ChatTokenLogprob>>,
    pub refusal: Option<Vec<ChatTokenLogprob>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTokenLogprob {
    pub token: String,
    pub bytes: Option<Vec<u8>>,
    pub logprob: f64,
    pub top_logprobs: Vec<ChatTopLogprob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTopLogprob {
    pub token: String,
    pub bytes: Option<Vec<u8>>,
    pub logprob: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
