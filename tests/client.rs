use open_ai_sdk::{
    resources::{
        chat::{ChatCompletionCreateParams, ChatMessage, ChatResponseFormatJsonSchema},
        responses::ResponseCreateParams,
    },
    Config, Error, JsonSchema, OpenAI,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn creates_response_with_auth_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_json(json!({
            "model": "gpt-4.1-mini",
            "input": "Hello"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_123",
            "object": "response",
            "created_at": 1,
            "model": "gpt-4.1-mini",
            "status": "completed",
            "output_text": "Hi"
        })))
        .mount(&server)
        .await;

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let response = client
        .responses()
        .create(ResponseCreateParams::new("gpt-4.1-mini", "Hello"))
        .await
        .expect("response");

    assert_eq!(response.id, "resp_123");
    assert_eq!(response.output_text.as_deref(), Some("Hi"));
}

#[tokio::test]
async fn creates_chat_completion() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_123",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4.1-mini",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Hello from Rust" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 3, "total_tokens": 4 }
        })))
        .mount(&server)
        .await;

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let completion = client
        .chat()
        .completions()
        .create(ChatCompletionCreateParams::new("gpt-4.1-mini").message(ChatMessage::user("Hello")))
        .await
        .expect("completion");

    assert_eq!(completion.choices[0].message.role, "assistant");
}

#[tokio::test]
async fn creates_chat_completion_with_json_schema_response_format() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_json(json!({
            "model": "gpt-4.1-mini",
            "messages": [{ "role": "user", "content": "Extract event data." }],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "event",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "city": { "type": "string" }
                        },
                        "required": ["city"],
                        "additionalProperties": false
                    },
                    "strict": true
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_456",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4.1-mini",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "{\"city\":\"Paris\"}" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let schema = json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"],
        "additionalProperties": false
    });
    let params = ChatCompletionCreateParams::new("gpt-4.1-mini")
        .message(ChatMessage::user("Extract event data."))
        .response_format(
            open_ai_sdk::resources::chat::ChatResponseFormat::JsonSchema {
                json_schema: ChatResponseFormatJsonSchema::new("event", schema).strict(true),
            },
        );

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let completion = client
        .chat()
        .completions()
        .create(params)
        .await
        .expect("completion");

    assert_eq!(completion.id, "chatcmpl_456");
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[schemars(deny_unknown_fields)]
struct ParsedCity {
    city: String,
}

#[tokio::test]
async fn parses_chat_completion_into_typed_struct() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_json(json!({
            "model": "gpt-4.1-mini",
            "messages": [{ "role": "user", "content": "Extract city." }],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "parsed_city",
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "title": "ParsedCity",
                        "type": "object",
                        "properties": {
                            "city": { "type": "string" }
                        },
                        "required": ["city"],
                        "additionalProperties": false
                    }
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_parse",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4.1-mini",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "{\"city\":\"Paris\"}" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let parsed = client
        .chat()
        .completions()
        .parse::<ParsedCity>(
            ChatCompletionCreateParams::new("gpt-4.1-mini")
                .message(ChatMessage::user("Extract city.")),
            "parsed_city",
        )
        .await
        .expect("parsed");

    assert_eq!(parsed.parsed.city, "Paris");
    assert_eq!(parsed.completion.id, "chatcmpl_parse");
}

#[tokio::test]
async fn parse_returns_error_when_completion_has_no_choices() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_empty",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4.1-mini",
            "choices": []
        })))
        .mount(&server)
        .await;

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let error = client
        .chat()
        .completions()
        .parse::<ParsedCity>(
            ChatCompletionCreateParams::new("gpt-4.1-mini")
                .message(ChatMessage::user("Extract city.")),
            "parsed_city",
        )
        .await
        .expect_err("parse error");

    match error {
        Error::Parse(message) => assert!(message.contains("no choices")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn streams_typed_chat_completion_chunks() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4.1-mini\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"finish_reason\":null}],\"usage\":null}\n\n",
        "data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4.1-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}],\"usage\":null}\n\n",
        "data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4.1-mini\",\"choices\":[],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_json(json!({
            "model": "gpt-4.1-mini",
            "messages": [{ "role": "user", "content": "Hello" }],
            "stream": true
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let mut stream = client
        .chat()
        .completions()
        .stream_typed(
            ChatCompletionCreateParams::new("gpt-4.1-mini").message(ChatMessage::user("Hello")),
        )
        .await
        .expect("stream");

    let first = stream
        .next_chunk()
        .await
        .expect("chunk result")
        .expect("first chunk");
    assert_eq!(first.choices[0].delta.role.as_deref(), Some("assistant"));
    assert_eq!(first.choices[0].delta.content.as_deref(), Some("Hel"));

    let second = stream
        .next_chunk()
        .await
        .expect("chunk result")
        .expect("second chunk");
    assert_eq!(second.choices[0].delta.content.as_deref(), Some("lo"));

    let usage = stream
        .next_chunk()
        .await
        .expect("chunk result")
        .expect("usage chunk");
    assert_eq!(usage.choices.len(), 0);
    assert_eq!(usage.usage.expect("usage").total_tokens, Some(2));

    let done = stream.next_chunk().await.expect("done result");
    assert!(done.is_none());
}

#[tokio::test]
async fn converts_api_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": {
                "message": "No such model",
                "type": "invalid_request_error",
                "param": "model",
                "code": "model_not_found"
            }
        })))
        .mount(&server)
        .await;

    let client = OpenAI::new(Config::new("test-key").with_base_url(format!("{}/v1", server.uri())))
        .expect("client");
    let error = client
        .models()
        .retrieve("missing")
        .await
        .expect_err("error");

    match error {
        Error::Api { message, error, .. } => {
            assert_eq!(message, "No such model");
            assert_eq!(error.expect("api error").param.as_deref(), Some("model"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
