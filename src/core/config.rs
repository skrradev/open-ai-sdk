use std::{env, time::Duration};

use crate::core::{Error, Result};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub base_url: String,
    pub timeout: Duration,
}

impl Config {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
            project: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(600),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| Error::MissingApiKey)?;
        let mut config = Self::new(api_key);
        config.organization = env::var("OPENAI_ORG_ID").ok();
        config.project = env::var("OPENAI_PROJECT_ID").ok();
        if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
            config.base_url = base_url;
        }
        Ok(config)
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
