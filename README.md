# open_ai_sdk

Rust SDK for the OpenAI API, structured after the TypeScript SDK while using Rust-native typed parameters, builders, async methods, and `Result`-based errors.

This is an initial port foundation. It includes:

- `OpenAI::from_env()` and configurable clients
- request options for per-call headers, query params, timeout, idempotency key, and base URL
- typed resources for Responses, Chat Completions, Embeddings, Files, and Models
- streaming support for Server-Sent Events endpoints
- JSON escape hatches through `serde_json::Value` and `extra` maps

## Install

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
