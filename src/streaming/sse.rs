use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;

use crate::core::{Error, Result};

pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: String,
    pending: VecDeque<Result<SseEvent>>,
    done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

impl SseStream {
    pub(crate) fn new(response: reqwest::Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
            pending: VecDeque::new(),
            done: false,
        }
    }

    pub async fn next_json<T>(&mut self) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        while let Some(event) = self.next().await {
            let event = event?;
            if event.data == "[DONE]" {
                return Ok(None);
            }
            return serde_json::from_str(&event.data)
                .map(Some)
                .map_err(Error::Json);
        }
        Ok(None)
    }

    fn drain_events(&mut self) {
        while let Some(index) = self.buffer.find("\n\n") {
            let raw = self.buffer[..index].to_string();
            self.buffer = self.buffer[index + 2..].to_string();
            if let Some(event) = parse_event(&raw) {
                self.pending.push_back(Ok(event));
            }
        }
    }
}

impl Stream for SseStream {
    type Item = Result<SseEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Some(event) = this.pending.pop_front() {
            return Poll::Ready(Some(event));
        }
        if this.done {
            return Poll::Ready(None);
        }

        loop {
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    match std::str::from_utf8(&chunk) {
                        Ok(text) => this.buffer.push_str(text),
                        Err(error) => {
                            return Poll::Ready(Some(Err(Error::Stream(error.to_string()))));
                        }
                    }
                    this.drain_events();
                    if let Some(event) = this.pending.pop_front() {
                        return Poll::Ready(Some(event));
                    }
                }
                Poll::Ready(Some(Err(error))) => {
                    return Poll::Ready(Some(Err(Error::Http(error))));
                }
                Poll::Ready(None) => {
                    this.done = true;
                    if !this.buffer.trim().is_empty() {
                        if let Some(event) = parse_event(&this.buffer) {
                            this.buffer.clear();
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

fn parse_event(raw: &str) -> Option<SseEvent> {
    let mut event = None;
    let mut data = Vec::new();

    for line in raw.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(value) = line.strip_prefix("event:") {
            event = Some(value.trim_start().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push(value.trim_start().to_string());
        }
    }

    if data.is_empty() {
        return None;
    }

    Some(SseEvent {
        event,
        data: data.join("\n"),
    })
}

impl From<reqwest::Response> for SseStream {
    fn from(response: reqwest::Response) -> Self {
        Self::new(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiline_event() {
        let event =
            parse_event("event: response.output_text.delta\ndata: {\"a\":1}\ndata: {\"b\":2}\n")
                .expect("event");
        assert_eq!(event.event.as_deref(), Some("response.output_text.delta"));
        assert_eq!(event.data, "{\"a\":1}\n{\"b\":2}");
    }
}
