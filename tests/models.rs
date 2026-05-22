use open_ai_sdk::{
    models::{ids, ModelId},
    resources::responses::ResponseCreateParams,
};
use serde_json::json;

#[test]
fn model_id_serializes_as_string() {
    let model = ModelId::from(ids::GPT_4O);
    assert_eq!(serde_json::to_value(model).expect("json"), json!("gpt-4o"));
}

#[test]
fn response_params_accept_future_model_names() {
    let params = ResponseCreateParams::new("gpt-6", "Hello");
    let body = serde_json::to_value(params).expect("json");

    assert_eq!(body["model"], "gpt-6");
    assert_eq!(body["input"], "Hello");
}
