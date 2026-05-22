# open_ai_sdk

Rust SDK for the OpenAI API, structured after the TypeScript SDK while using Rust-native typed parameters, builders, async methods, and `Result`-based errors.

This is an early Rust SDK for the OpenAI API. It currently includes:

- `OpenAI::from_env()` and configurable clients
- request options for per-call headers, query params, timeout, idempotency key, and base URL
- typed resources for Responses, Chat Completions, Embeddings, Files, and Models
- flexible string-backed model IDs with convenience constants
- broad Chat Completions parameter coverage, including tools, tool choice, logprobs, audio, prediction, service tier, cache controls, verbosity, and GPT-5 reasoning effort
- chat structured outputs from hand-written JSON Schema or `#[derive(JsonSchema)]`
- `parse::<T>()` helpers that return typed Rust structs directly
- raw SSE streaming and typed Chat Completions streaming chunks
- token usage exposure for non-streaming and streaming responses
- JSON escape hatches through `serde_json::Value` and `extra` maps

## Install

From GitHub using the latest release tag:

```toml
[dependencies]
open_ai_sdk = { git = "https://github.com/skrradev/open-ai-sdk", tag = "v0.1.0" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

From GitHub using the current `master` branch:

```toml
[dependencies]
open_ai_sdk = { git = "https://github.com/skrradev/open-ai-sdk", branch = "master" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For local development:

```toml
[dependencies]
open_ai_sdk = { path = "." }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Crate layout

```text
src/
  client/       Public OpenAI client entry point.
  core/         Config, errors, request options, and internal HTTP transport.
  resources/    Endpoint modules such as responses, chat, files, models.
  streaming/    Server-Sent Events stream parser and helpers.
  types/        Shared API envelope and utility types.
  prelude.rs    Common imports for application code.
```

New endpoint ports should generally add one module under `resources/`, put shared DTOs in `types/` only when multiple resources need them, and keep request execution inside `core::HttpClient`.

## Models

Model IDs are intentionally string-backed so new OpenAI models can be used immediately without a crate release.

```rust,no_run
use open_ai_sdk::{models::ids, resources::responses::ResponseCreateParams};

let current = ResponseCreateParams::new(ids::GPT_5_4, "Hello");
let future = ResponseCreateParams::new("gpt-6", "Hello");
```

## Responses API

```rust,no_run
use open_ai_sdk::{OpenAI, resources::responses::ResponseCreateParams};

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;

    let response = client
        .responses()
        .create(ResponseCreateParams::new("gpt-4.1-mini", "Write a haiku about Rust."))
        .await?;

    println!("{response:#?}");
    Ok(())
}
```

## Chat Completions

```rust,no_run
use open_ai_sdk::{
    OpenAI,
    resources::chat::{ChatCompletionCreateParams, ChatMessage},
};

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;

    let completion = client
        .chat()
        .completions()
        .create(
            ChatCompletionCreateParams::new("gpt-4.1-mini")
                .message(ChatMessage::user("Say hello from Rust.")),
        )
        .await?;

    println!("{completion:#?}");
    Ok(())
}
```

Chat Completions supports the common tuning and tool parameters exposed by the official SDK:

```rust,no_run
use open_ai_sdk::{
    models::ids,
    resources::chat::{
        ChatCompletionCreateParams, ChatFunctionDefinition, ChatMessage, ChatReasoningEffort,
        ChatTool, ChatToolChoice,
    },
};
use serde_json::json;

let params = ChatCompletionCreateParams::new(ids::GPT_5_4)
    .message(ChatMessage::developer("Answer with concise JSON."))
    .message(ChatMessage::user("What is the weather in Almaty?"))
    .reasoning_effort(ChatReasoningEffort::Medium)
    .temperature(0.2)
    .max_completion_tokens(500)
    .tool(ChatTool::function(
        ChatFunctionDefinition::new("get_weather")
            .description("Get weather for a city")
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
    .tool_choice(ChatToolChoice::auto());
```

## Chat structured outputs

```rust,no_run
use open_ai_sdk::{
    resources::chat::{ChatCompletionCreateParams, ChatMessage},
    OpenAI,
};
use serde_json::json;

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;

    let completion = client
        .chat()
        .completions()
        .create(
            ChatCompletionCreateParams::new("gpt-4.1-mini")
                .message(ChatMessage::user("Extract the city and date: Paris, June 3."))
                .json_schema(
                    "event",
                    json!({
                        "type": "object",
                        "properties": {
                            "city": { "type": "string" },
                            "date": { "type": "string" }
                        },
                        "required": ["city", "date"],
                        "additionalProperties": false
                    }),
                ),
        )
        .await?;

    println!("{completion:#?}");
    Ok(())
}
```

You can also derive JSON Schema from a Rust struct:

```rust,no_run
use open_ai_sdk::{
    resources::chat::{ChatCompletionCreateParams, ChatMessage},
    JsonSchema,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Event {
    city: String,
    date: String,
}

let params = ChatCompletionCreateParams::new("gpt-4.1-mini")
    .message(ChatMessage::user("Extract the city and date: Paris, June 3."))
    .json_schema_for::<Event>("event");
```

To call the API and deserialize the first assistant message directly into a Rust type:

```rust,no_run
use open_ai_sdk::{
    resources::chat::{ChatCompletionCreateParams, ChatMessage},
    JsonSchema, OpenAI,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct Event {
    city: String,
    date: String,
}

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;

    let parsed = client
        .chat()
        .completions()
        .parse::<Event>(
            ChatCompletionCreateParams::new("gpt-4o")
                .message(ChatMessage::user("Extract the city and date: Paris, June 3.")),
            "event",
        )
        .await?;

    println!("{:?}", parsed.parsed);

    if let Some(usage) = parsed.completion.usage {
        println!("prompt={:?} completion={:?} total={:?}",
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.total_tokens,
        );
    }

    Ok(())
}
```

## Chat streaming

Use raw SSE when you need event-level access:

```rust,no_run
use futures_util::StreamExt;
use open_ai_sdk::{resources::chat::{ChatCompletionCreateParams, ChatMessage}, OpenAI};

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;
    let mut stream = client
        .chat()
        .completions()
        .stream(ChatCompletionCreateParams::new("gpt-4o").message(ChatMessage::user("Hello")))
        .await?;

    while let Some(event) = stream.next().await {
        println!("{:?}", event?);
    }

    Ok(())
}
```

Use typed chunks for normal Chat Completions streaming:

```rust,no_run
use open_ai_sdk::{
    resources::chat::{ChatCompletionCreateParams, ChatMessage, ChatStreamOptions},
    OpenAI,
};

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;
    let mut stream = client
        .chat()
        .completions()
        .stream_typed(
            ChatCompletionCreateParams::new("gpt-4o")
                .message(ChatMessage::user("Write one sentence."))
                .stream_options(ChatStreamOptions::new().include_usage(true)),
        )
        .await?;

    while let Some(chunk) = stream.next_chunk().await? {
        for choice in &chunk.choices {
            if let Some(delta) = &choice.delta.content {
                print!("{delta}");
            }
        }
        if let Some(usage) = chunk.usage {
            eprintln!("total tokens: {:?}", usage.total_tokens);
        }
    }

    Ok(())
}
```

## Files

```rust,no_run
use open_ai_sdk::{OpenAI, resources::files::FilePurpose};

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;
    let file = client.files().upload_path("data.jsonl", FilePurpose::FineTune).await?;
    println!("{}", file.id);
    Ok(())
}
```

## Live integration tests

The repository includes ignored tests that hit the real API. Create a local `.env` file:

```env
OPENAI_API_KEY=...
OPENAI_BASE_URL=https://api.openai.com/v1
```

Then run:

```bash
cargo test --test live_chat_structured_outputs -- --ignored --nocapture
```

Those tests currently verify:

- `gpt-5.4` structured outputs with medium reasoning
- `gpt-4o` structured outputs
- `parse::<T>()` against a real nested schema
- token usage on parsed completions
- token usage chunks in typed streaming
