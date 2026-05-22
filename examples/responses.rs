use open_ai_sdk::{resources::responses::ResponseCreateParams, OpenAI};

#[tokio::main]
async fn main() -> open_ai_sdk::Result<()> {
    let client = OpenAI::from_env()?;
    let response = client
        .responses()
        .create(ResponseCreateParams::new(
            "gpt-4.1-mini",
            "Explain ownership in Rust in one paragraph.",
        ))
        .await?;

    println!("{response:#?}");
    Ok(())
}
