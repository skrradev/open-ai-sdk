use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    core::{HttpClient, Result},
    types::{Deleted, List},
};

#[derive(Clone, Debug)]
pub struct ModelsResource {
    client: Arc<HttpClient>,
}

impl ModelsResource {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<List<Model>> {
        self.client.get("/models", None).await
    }

    pub async fn retrieve(&self, model: impl AsRef<str>) -> Result<Model> {
        self.client
            .get(&format!("/models/{}", model.as_ref()), None)
            .await
    }

    pub async fn delete(&self, model: impl AsRef<str>) -> Result<Deleted> {
        self.client
            .delete(&format!("/models/{}", model.as_ref()), None)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: Option<u64>,
    pub owned_by: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
