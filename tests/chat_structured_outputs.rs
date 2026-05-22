use open_ai_sdk::resources::chat::{
    ChatAllowedTools, ChatAllowedToolsMode, ChatApproximateLocation, ChatAudioFormat,
    ChatAudioParam, ChatCompletionCreateParams, ChatFunctionDefinition, ChatMessage, ChatModality,
    ChatPredictionContent, ChatReasoningEffort, ChatResponseFormat, ChatResponseFormatJsonSchema,
    ChatSearchContextSize, ChatServiceTier, ChatStreamOptions, ChatTool, ChatToolChoice,
    ChatVerbosity, ChatVoice, ChatWebSearchOptions, ChatWebSearchUserLocation,
    PromptCacheRetention,
};
use open_ai_sdk::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct Event {
    city: String,
    date: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct NestedPlan {
    name: String,
    owner: NestedOwner,
    confidence: NestedConfidence,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct NestedOwner {
    email: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum NestedConfidence {
    Low,
    Medium,
    High,
}

#[test]
fn serializes_json_schema_response_format() {
    let params = ChatCompletionCreateParams::new("gpt-4.1-mini")
        .message(ChatMessage::user("Extract event data."))
        .json_schema(
            "event",
            json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                },
                "required": ["city"],
                "additionalProperties": false
            }),
        );

    let body = serde_json::to_value(params).expect("json");

    assert_eq!(
        body["response_format"],
        json!({
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
                }
            }
        })
    );
}

#[test]
fn serializes_strict_json_schema_response_format() {
    let format = ChatResponseFormat::JsonSchema {
        json_schema: ChatResponseFormatJsonSchema::new(
            "event",
            json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                },
                "required": ["city"],
                "additionalProperties": false
            }),
        )
        .description("Extracted event data")
        .strict(true),
    };

    let body = serde_json::to_value(format).expect("json");

    assert_eq!(
        body,
        json!({
            "type": "json_schema",
            "json_schema": {
                "name": "event",
                "description": "Extracted event data",
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
        })
    );
}

#[test]
fn serializes_json_object_response_format() {
    let body = serde_json::to_value(ChatResponseFormat::json_object()).expect("json");

    assert_eq!(body, json!({ "type": "json_object" }));
}

#[test]
fn generates_json_schema_response_format_from_struct() {
    let params = ChatCompletionCreateParams::new("gpt-4.1-mini")
        .message(ChatMessage::user("Extract event data."))
        .json_schema_for::<Event>("event");

    let body = serde_json::to_value(params).expect("json");
    let json_schema = &body["response_format"]["json_schema"];

    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(json_schema["name"], "event");
    assert_eq!(json_schema["schema"]["type"], "object");
    assert_eq!(
        json_schema["schema"]["properties"]["city"]["type"],
        "string"
    );
    assert_eq!(
        json_schema["schema"]["properties"]["date"]["type"],
        "string"
    );
    assert_eq!(json_schema["schema"]["required"], json!(["city", "date"]));
    assert_eq!(json_schema["schema"]["additionalProperties"], false);
}

#[test]
fn serializes_medium_reasoning_effort() {
    let params = ChatCompletionCreateParams::new("gpt-5.4")
        .message(ChatMessage::user("Extract event data."))
        .reasoning_effort(ChatReasoningEffort::Medium)
        .json_schema_for::<Event>("event");

    let body = serde_json::to_value(params).expect("json");

    assert_eq!(body["reasoning_effort"], "medium");
}

#[test]
fn generated_nested_schema_marks_top_level_fields_required() {
    let params = ChatCompletionCreateParams::new("gpt-5.4")
        .message(ChatMessage::user("Extract plan."))
        .json_schema_for::<NestedPlan>("nested_plan");

    let body = serde_json::to_value(params).expect("json");
    let schema = &body["response_format"]["json_schema"]["schema"];

    assert_eq!(schema["required"], json!(["name", "owner", "confidence"]));
}

#[test]
fn serializes_official_style_chat_tuning_params() {
    let params = ChatCompletionCreateParams::new("gpt-5.4")
        .message(ChatMessage::user("Tune this request."))
        .temperature(0.2)
        .top_p(0.9)
        .frequency_penalty(0.1)
        .presence_penalty(0.2)
        .max_completion_tokens(500)
        .max_tokens(400)
        .reasoning_effort(ChatReasoningEffort::Medium)
        .verbosity(ChatVerbosity::Low)
        .seed(123)
        .n(2)
        .stop(vec!["END", "DONE"])
        .store(true)
        .user("user_123")
        .safety_identifier("safe_123")
        .prompt_cache_key("cache-key")
        .prompt_cache_retention(PromptCacheRetention::TwentyFourHours)
        .service_tier(ChatServiceTier::Priority)
        .metadata_pair("feature", "chat")
        .logprobs(true)
        .top_logprobs(3)
        .logit_bias_token("42", -5.0)
        .stream_options(
            ChatStreamOptions::new()
                .include_usage(true)
                .include_obfuscation(false),
        )
        .modality(ChatModality::Text)
        .audio(ChatAudioParam::new(
            ChatAudioFormat::Mp3,
            ChatVoice::id("voice_123"),
        ))
        .prediction(ChatPredictionContent::text("known prefix"))
        .parallel_tool_calls(true)
        .tool(ChatTool::function(
            ChatFunctionDefinition::new("get_weather")
                .description("Get weather")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    },
                    "required": ["city"],
                    "additionalProperties": false
                }))
                .strict(true),
        ))
        .tool_choice(ChatToolChoice::function("get_weather"))
        .web_search_options(
            ChatWebSearchOptions::new()
                .search_context_size(ChatSearchContextSize::Low)
                .user_location(ChatWebSearchUserLocation::approximate(
                    ChatApproximateLocation::new()
                        .city("Almaty")
                        .country("KZ")
                        .timezone("Asia/Almaty"),
                )),
        )
        .extra("custom_future_param", json!("kept"));

    let body = serde_json::to_value(params).expect("json");

    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["top_p"], 0.9);
    assert_eq!(body["frequency_penalty"], 0.1);
    assert_eq!(body["presence_penalty"], 0.2);
    assert_eq!(body["max_completion_tokens"], 500);
    assert_eq!(body["max_tokens"], 400);
    assert_eq!(body["reasoning_effort"], "medium");
    assert_eq!(body["verbosity"], "low");
    assert_eq!(body["seed"], 123);
    assert_eq!(body["n"], 2);
    assert_eq!(body["stop"], json!(["END", "DONE"]));
    assert_eq!(body["store"], true);
    assert_eq!(body["user"], "user_123");
    assert_eq!(body["safety_identifier"], "safe_123");
    assert_eq!(body["prompt_cache_key"], "cache-key");
    assert_eq!(body["prompt_cache_retention"], "24h");
    assert_eq!(body["service_tier"], "priority");
    assert_eq!(body["metadata"]["feature"], "chat");
    assert_eq!(body["logprobs"], true);
    assert_eq!(body["top_logprobs"], 3);
    assert_eq!(body["logit_bias"]["42"], -5.0);
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert_eq!(body["stream_options"]["include_obfuscation"], false);
    assert_eq!(body["modalities"], json!(["text"]));
    assert_eq!(body["audio"]["format"], "mp3");
    assert_eq!(body["audio"]["voice"]["id"], "voice_123");
    assert_eq!(body["prediction"]["type"], "content");
    assert_eq!(body["prediction"]["content"], "known prefix");
    assert_eq!(body["parallel_tool_calls"], true);
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["function"]["name"], "get_weather");
    assert_eq!(body["tools"][0]["function"]["strict"], true);
    assert_eq!(body["tool_choice"]["type"], "function");
    assert_eq!(body["tool_choice"]["function"]["name"], "get_weather");
    assert_eq!(body["web_search_options"]["search_context_size"], "low");
    assert_eq!(
        body["web_search_options"]["user_location"]["type"],
        "approximate"
    );
    assert_eq!(
        body["web_search_options"]["user_location"]["approximate"]["city"],
        "Almaty"
    );
    assert_eq!(body["custom_future_param"], "kept");
}

#[test]
fn serializes_tool_choice_modes_and_allowed_tools() {
    let auto = serde_json::to_value(ChatToolChoice::auto()).expect("json");
    assert_eq!(auto, json!("auto"));

    let allowed = serde_json::to_value(ChatToolChoice::allowed(ChatAllowedTools::new(
        ChatAllowedToolsMode::Required,
        vec![json!({
            "type": "function",
            "function": { "name": "get_weather" }
        })],
    )))
    .expect("json");

    assert_eq!(
        allowed,
        json!({
            "type": "allowed_tools",
            "allowed_tools": {
                "mode": "required",
                "tools": [{
                    "type": "function",
                    "function": { "name": "get_weather" }
                }]
            }
        })
    );
}
