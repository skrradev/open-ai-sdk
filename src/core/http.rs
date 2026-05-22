use reqwest::{header, Method, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    core::{ApiErrorEnvelope, Config, Error, RequestOptions, Result},
    streaming::SseStream,
};

#[derive(Debug)]
pub(crate) struct HttpClient {
    http: reqwest::Client,
    config: Config,
}

impl HttpClient {
    pub fn new(config: Config) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(Error::Http)?;
        Ok(Self { http, config })
    }

    pub async fn get<T>(&self, path: &str, options: Option<RequestOptions>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.send_json::<(), T>(Method::GET, path, None, options)
            .await
    }

    pub async fn delete<T>(&self, path: &str, options: Option<RequestOptions>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.send_json::<(), T>(Method::DELETE, path, None, options)
            .await
    }

    pub async fn post<P, T>(
        &self,
        path: &str,
        body: P,
        options: Option<RequestOptions>,
    ) -> Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        self.send_json(Method::POST, path, Some(body), options)
            .await
    }

    pub async fn post_stream<P>(
        &self,
        path: &str,
        body: P,
        options: Option<RequestOptions>,
    ) -> Result<SseStream>
    where
        P: Serialize,
    {
        let request = self
            .request(Method::POST, path, options)?
            .json(&body)
            .header(header::ACCEPT, "text/event-stream");
        let response = request.send().await?;
        self.error_for_status(response.status(), response)
            .await
            .map(SseStream::new)
    }

    pub async fn send_multipart<T>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
        options: Option<RequestOptions>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .request(Method::POST, path, options)?
            .multipart(form)
            .send()
            .await?;
        self.json_response(response).await
    }

    pub async fn get_bytes(
        &self,
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<bytes::Bytes> {
        let response = self.request(Method::GET, path, options)?.send().await?;
        let response = self.error_for_status(response.status(), response).await?;
        response.bytes().await.map_err(Error::Http)
    }

    async fn send_json<P, T>(
        &self,
        method: Method,
        path: &str,
        body: Option<P>,
        options: Option<RequestOptions>,
    ) -> Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let mut request = self.request(method, path, options)?;
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await?;
        self.json_response(response).await
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<RequestBuilder> {
        let options = options.unwrap_or_default();
        let base_url = options
            .base_url
            .as_deref()
            .unwrap_or(self.config.base_url.as_str())
            .trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let url = format!("{base_url}/{path}");

        let mut request = self
            .http
            .request(method, url)
            .bearer_auth(&self.config.api_key)
            .header(
                header::USER_AGENT,
                concat!("open_ai_sdk-rust/", env!("CARGO_PKG_VERSION")),
            );

        if let Some(org) = &self.config.organization {
            request = request.header("OpenAI-Organization", org);
        }
        if let Some(project) = &self.config.project {
            request = request.header("OpenAI-Project", project);
        }
        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }
        if let Some(key) = options.idempotency_key {
            request = request.header("Idempotency-Key", key);
        }
        for (key, value) in options.headers {
            request = request.header(key, value);
        }
        if !options.query.is_empty() {
            request = request.query(&options.query);
        }

        Ok(request)
    }

    async fn json_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let status = response.status();
        let response = self.error_for_status(status, response).await?;
        response.json::<T>().await.map_err(Error::Http)
    }

    async fn error_for_status(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> Result<reqwest::Response> {
        if status.is_success() {
            return Ok(response);
        }

        let text = response.text().await.unwrap_or_default();
        let parsed = serde_json::from_str::<ApiErrorEnvelope>(&text).ok();
        let api_error = parsed.and_then(|envelope| envelope.error);
        let message = api_error
            .as_ref()
            .map(|error| error.message.clone())
            .unwrap_or(text);

        Err(Error::Api {
            status,
            message,
            error: api_error,
        })
    }
}
