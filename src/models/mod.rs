mod id;

pub use id::ModelId;

pub mod ids {
    pub const GPT_5_5: &str = "gpt-5.5";
    pub const GPT_5_4: &str = "gpt-5.4";
    pub const GPT_5_4_MINI: &str = "gpt-5.4-mini";
    pub const GPT_5_4_NANO: &str = "gpt-5.4-nano";
    pub const GPT_5_2: &str = "gpt-5.2";
    pub const GPT_5_1: &str = "gpt-5.1";
    pub const GPT_5: &str = "gpt-5";
    pub const GPT_5_MINI: &str = "gpt-5-mini";
    pub const GPT_5_NANO: &str = "gpt-5-nano";
    pub const GPT_4_1: &str = "gpt-4.1";
    pub const GPT_4_1_MINI: &str = "gpt-4.1-mini";
    pub const GPT_4O: &str = "gpt-4o";
    pub const GPT_4O_MINI: &str = "gpt-4o-mini";
    pub const TEXT_EMBEDDING_3_LARGE: &str = "text-embedding-3-large";
    pub const TEXT_EMBEDDING_3_SMALL: &str = "text-embedding-3-small";
}
