use std::{env, fs};

use open_ai_sdk::{
    models::ids,
    resources::chat::{
        ChatCompletionCreateParams, ChatMessage, ChatMessageContent, ChatReasoningEffort,
        ChatStreamOptions,
    },
    Config, JsonSchema, OpenAI,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct ReleasePlan {
    project: String,
    launch: LaunchDetails,
    summary: String,
    audience: Vec<AudienceSegment>,
    milestones: Vec<Milestone>,
    risks: Vec<Risk>,
    owner: Person,
    confidence: Confidence,
    follow_up_required: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct LaunchDetails {
    /// ISO 8601 calendar date, formatted as YYYY-MM-DD.
    date: String,
    city: String,
    venue: String,
    expected_attendees: u32,
    remote_available: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct AudienceSegment {
    name: String,
    size: u32,
    priority: Priority,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct Milestone {
    title: String,
    due_date: String,
    owner: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct Risk {
    name: String,
    severity: Severity,
    mitigation: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct Person {
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Confidence {
    Low,
    Medium,
    High,
}

#[ignore = "requires OPENAI_API_KEY in .env and makes a real API request"]
#[tokio::test]
async fn chat_structured_output_with_gpt54_medium_reasoning() -> open_ai_sdk::Result<()> {
    let client = live_client()?;
    let completion = client
        .chat()
        .completions()
        .create(
            ChatCompletionCreateParams::new(ids::GPT_5_4)
                .message(ChatMessage::developer(
                    "Return only JSON matching the supplied schema. Include every required field. Use ISO 8601 YYYY-MM-DD for dates. Set confidence to high when all requested facts are explicitly present. Do not invent facts beyond the user message.",
                ))
                .message(ChatMessage::user(release_plan_prompt()))
                .reasoning_effort(ChatReasoningEffort::Medium)
                .json_schema_for::<ReleasePlan>("release_plan"),
        )
        .await?;

    assert_release_plan(completion.choices[0].message.content.clone());

    Ok(())
}

#[ignore = "requires OPENAI_API_KEY in .env and makes a real API request"]
#[tokio::test]
async fn chat_structured_output_with_gpt4o() -> open_ai_sdk::Result<()> {
    let client = live_client()?;
    let completion = client
        .chat()
        .completions()
        .create(
            ChatCompletionCreateParams::new(ids::GPT_4O)
                .message(ChatMessage::developer(
                    "Return only JSON matching the supplied schema. Include every required field. Use ISO 8601 YYYY-MM-DD for dates. Set confidence to high when all requested facts are explicitly present. Do not invent facts beyond the user message.",
                ))
                .message(ChatMessage::user(release_plan_prompt()))
                .json_schema_for::<ReleasePlan>("release_plan"),
        )
        .await?;

    assert_release_plan(completion.choices[0].message.content.clone());

    Ok(())
}

#[ignore = "requires OPENAI_API_KEY in .env and makes a real API request"]
#[tokio::test]
async fn chat_parse_helper_with_gpt54_medium_reasoning() -> open_ai_sdk::Result<()> {
    let client = live_client()?;
    let parsed = client
        .chat()
        .completions()
        .parse::<ReleasePlan>(
            ChatCompletionCreateParams::new(ids::GPT_5_4)
                .message(ChatMessage::developer(
                    "Return only JSON matching the supplied schema. Include every required field. Use ISO 8601 YYYY-MM-DD for dates. Set confidence to high when all requested facts are explicitly present. Do not invent facts beyond the user message.",
                ))
                .message(ChatMessage::user(release_plan_prompt()))
                .reasoning_effort(ChatReasoningEffort::Medium),
            "release_plan",
        )
        .await?;

    assert_release_plan_value(&parsed.parsed);
    assert!(parsed.completion.model.starts_with(ids::GPT_5_4));
    assert!(!parsed.completion.choices.is_empty());
    assert_usage_present(parsed.completion.usage.as_ref());

    Ok(())
}

#[ignore = "requires OPENAI_API_KEY in .env and makes a real API request"]
#[tokio::test]
async fn chat_typed_stream_receives_usage_chunk() -> open_ai_sdk::Result<()> {
    let client = live_client()?;
    let mut stream = client
        .chat()
        .completions()
        .stream_typed(
            ChatCompletionCreateParams::new(ids::GPT_4O)
                .message(ChatMessage::user("Reply with exactly one short sentence."))
                .stream_options(ChatStreamOptions::new().include_usage(true)),
        )
        .await?;

    let mut content = String::new();
    let mut usage = None;
    while let Some(chunk) = stream.next_chunk().await? {
        for choice in &chunk.choices {
            if let Some(delta) = &choice.delta.content {
                content.push_str(delta);
            }
        }
        if chunk.usage.is_some() {
            usage = chunk.usage;
        }
    }

    assert!(!content.trim().is_empty());
    assert_usage_present(usage.as_ref());

    Ok(())
}

fn live_client() -> open_ai_sdk::Result<OpenAI> {
    load_dotenv();

    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env");
    let mut config = Config::new(api_key);
    if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
        if !base_url.trim().is_empty() {
            config = config.with_base_url(base_url);
        }
    }
    if let Ok(org_id) = env::var("OPENAI_ORG_ID") {
        if !org_id.trim().is_empty() {
            config = config.with_organization(org_id);
        }
    }
    if let Ok(project_id) = env::var("OPENAI_PROJECT_ID") {
        if !project_id.trim().is_empty() {
            config = config.with_project(project_id);
        }
    }

    OpenAI::new(config)
}

fn release_plan_prompt() -> &'static str {
    "Create a release plan from this brief. Required fields: project, launch, summary, audience, milestones, risks, owner, confidence, follow_up_required. Brief: OpenAI Rust SDK beta launches at QazaqTech Hub in Almaty on May 22, 2026. The event expects 42 in-person attendees and remote participation is available. Primary audience: Rust backend developers, 30 people, high priority. Secondary audience: platform engineering leads, 12 people, medium priority. Milestones: publish crate docs by May 10, 2026 owned by Aigerim; finish structured-output examples by May 12, 2026 owned by Timur; run API integration tests by May 15, 2026 owned by Dana. Main risks: API model availability is medium severity and should be mitigated with fallback model configuration; schema drift is high severity and should be mitigated with live structured-output tests. Owner: Dana Kim, dana@example.com. Confidence: high. Follow-up is required."
}

fn assert_release_plan(content: ChatMessageContent) {
    let content = match content {
        ChatMessageContent::Text(content) => content,
        ChatMessageContent::Parts(_) => panic!("expected text content"),
    };
    let extracted: ReleasePlan = serde_json::from_str(&content).expect("valid structured JSON");
    assert_release_plan_value(&extracted);
}

fn assert_release_plan_value(extracted: &ReleasePlan) {
    let project = extracted.project.to_lowercase();
    assert!(project.contains("rust sdk") || extracted.summary.to_lowercase().contains("rust sdk"));
    assert_eq!(extracted.launch.city, "Almaty");
    assert_eq!(extracted.launch.venue, "QazaqTech Hub");
    assert_eq!(extracted.launch.date, "2026-05-22");
    assert_eq!(extracted.launch.expected_attendees, 42);
    assert!(extracted.launch.remote_available);
    assert_eq!(extracted.audience.len(), 2);
    assert_eq!(extracted.audience[0].priority, Priority::High);
    assert_eq!(extracted.milestones.len(), 3);
    assert!(extracted.milestones.iter().any(|milestone| milestone
        .title
        .to_lowercase()
        .contains("integration test")
        && milestone.due_date == "2026-05-15"
        && milestone.owner == "Dana"));
    assert_eq!(extracted.risks.len(), 2);
    assert!(
        extracted
            .risks
            .iter()
            .any(|risk| risk.name.to_lowercase().contains("schema")
                && risk.severity == Severity::High)
    );
    assert_eq!(extracted.owner.email, "dana@example.com");
    assert_eq!(extracted.confidence, Confidence::High);
    assert!(extracted.follow_up_required);
}

fn assert_usage_present(usage: Option<&open_ai_sdk::resources::chat::ChatUsage>) {
    let usage = usage.expect("usage should be present");
    let prompt_tokens = usage
        .prompt_tokens
        .expect("prompt tokens should be present");
    let completion_tokens = usage
        .completion_tokens
        .expect("completion tokens should be present");
    let total_tokens = usage.total_tokens.expect("total tokens should be present");

    assert!(prompt_tokens > 0);
    assert!(completion_tokens > 0);
    assert!(total_tokens >= prompt_tokens + completion_tokens);
}

fn load_dotenv() {
    let Ok(contents) = fs::read_to_string(".env") else {
        return;
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if env::var_os(key).is_none() {
            env::set_var(key, value.trim_matches('"'));
        }
    }
}
