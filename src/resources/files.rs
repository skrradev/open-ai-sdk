use std::{collections::BTreeMap, path::Path, sync::Arc};

use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{
    core::{HttpClient, Result},
    types::{Deleted, List},
};

#[derive(Clone, Debug)]
pub struct FilesResource {
    client: Arc<HttpClient>,
}

impl FilesResource {
    pub(crate) fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<List<FileObject>> {
        self.client.get("/files", None).await
    }

    pub async fn retrieve(&self, file_id: impl AsRef<str>) -> Result<FileObject> {
        self.client
            .get(&format!("/files/{}", file_id.as_ref()), None)
            .await
    }

    pub async fn delete(&self, file_id: impl AsRef<str>) -> Result<Deleted> {
        self.client
            .delete(&format!("/files/{}", file_id.as_ref()), None)
            .await
    }

    pub async fn upload_path(
        &self,
        path: impl AsRef<Path>,
        purpose: FilePurpose,
    ) -> Result<FileObject> {
        let path = path.as_ref();
        let bytes = fs::read(path).await?;
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file")
            .to_string();
        self.upload_bytes(bytes, filename, purpose).await
    }

    pub async fn upload_bytes(
        &self,
        bytes: impl Into<Vec<u8>>,
        filename: impl Into<String>,
        purpose: FilePurpose,
    ) -> Result<FileObject> {
        let part = multipart::Part::bytes(bytes.into()).file_name(filename.into());
        let form = multipart::Form::new()
            .part("file", part)
            .text("purpose", purpose.as_str().to_string());
        self.client.send_multipart("/files", form, None).await
    }

    pub async fn content(&self, file_id: impl AsRef<str>) -> Result<bytes::Bytes> {
        self.client
            .get_bytes(&format!("/files/{}/content", file_id.as_ref()), None)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileObject {
    pub id: String,
    pub object: String,
    pub bytes: Option<u64>,
    pub created_at: Option<u64>,
    pub filename: Option<String>,
    pub purpose: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilePurpose {
    #[serde(rename = "assistants")]
    Assistants,
    #[serde(rename = "batch")]
    Batch,
    #[serde(rename = "fine-tune")]
    FineTune,
    #[serde(rename = "vision")]
    Vision,
    #[serde(rename = "user_data")]
    UserData,
    #[serde(rename = "eval")]
    Eval,
}

impl FilePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assistants => "assistants",
            Self::Batch => "batch",
            Self::FineTune => "fine-tune",
            Self::Vision => "vision",
            Self::UserData => "user_data",
            Self::Eval => "eval",
        }
    }
}
