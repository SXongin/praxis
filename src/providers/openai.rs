use crate::protocol::{ContentBlock, Message, Role};

use super::{ChatStream, Provider, ProviderError};

pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com".into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };
                let text = m
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                serde_json::json!({
                    "role": role,
                    "content": text,
                })
            })
            .collect()
    }

    fn tools_to_openai(tools: &[super::ToolDef]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }
}

#[derive(Debug, serde::Deserialize)]
struct ChatChunk {
    choices: Vec<Choice>,
}

#[derive(Debug, serde::Deserialize)]
struct Choice {
    delta: Option<Delta>,
}

#[derive(Debug, serde::Deserialize)]
struct Delta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, serde::Deserialize)]
struct ToolCallDelta {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    r#type: Option<String>,
    function: Option<FunctionDelta>,
}

#[derive(Debug, serde::Deserialize)]
struct FunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Default)]
struct ToolCallAccum {
    id: String,
    name: String,
    arguments: String,
}

impl Provider for OpenAiProvider {
    fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<super::ToolDef>,
    ) -> impl std::future::Future<Output = Result<ChatStream, ProviderError>> + Send {
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let openai_messages = Self::messages_to_openai(&messages);
        let openai_tools = Self::tools_to_openai(&tools);
        let base_url = self.base_url.clone();
        async move {
            let mut body = serde_json::json!({
                "model": "gpt-4o",
                "messages": openai_messages,
                "stream": true,
            });
            if !openai_tools.is_empty() {
                body["tools"] = serde_json::json!(openai_tools);
            }

            let url = format!("{}/v1/chat/completions", base_url);
            let mut last_error = None;

            for attempt in 0..3 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        2u64.pow(attempt as u32) * 100,
                    ))
                    .await;
                }

                let response = match client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&body)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        last_error = Some(ProviderError::Other(e.to_string()));
                        continue;
                    }
                };

                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    last_error = Some(ProviderError::Other(format!(
                        "OpenAI API error {}: {}",
                        status, text
                    )));
                    continue;
                }

                let stream = response.bytes_stream();
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

                tokio::spawn(async move {
                    use futures::StreamExt;
                    futures::pin_mut!(stream);
                    let mut tool_call_buf: Vec<ToolCallAccum> = vec![];
                    while let Some(chunk) = stream.next().await {
                        let chunk = match chunk {
                            Ok(c) => c,
                            Err(_) => break,
                        };
                        let text = String::from_utf8_lossy(&chunk);
                        for line in text.lines() {
                            let data = line.strip_prefix("data: ").unwrap_or(line);
                            if data == "[DONE]" {
                                for tc in tool_call_buf.drain(..) {
                                    let _ = tx.send(ContentBlock::ToolCall {
                                        id: tc.id,
                                        name: tc.name,
                                        arguments: tc.arguments,
                                    });
                                }
                                return;
                            }
                            if let Ok(parsed) = serde_json::from_str::<ChatChunk>(data) {
                                for choice in parsed.choices {
                                    if let Some(delta) = choice.delta {
                                        if let Some(text) = delta.content {
                                            if !text.is_empty() {
                                                let _ = tx.send(ContentBlock::Text { text });
                                            }
                                        }
                                        if let Some(tool_calls) = delta.tool_calls {
                                            for tc in tool_calls {
                                                let idx = tc.index;
                                                while tool_call_buf.len() <= idx {
                                                    tool_call_buf.push(ToolCallAccum::default());
                                                }
                                                let buf = &mut tool_call_buf[idx];
                                                if let Some(id) = tc.id {
                                                    buf.id = id;
                                                }
                                                if let Some(ref func) = tc.function {
                                                    if let Some(ref name) = func.name {
                                                        buf.name = name.clone();
                                                    }
                                                    if let Some(ref args) = func.arguments {
                                                        buf.arguments.push_str(args);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                });

                let stream: ChatStream =
                    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx));
                return Ok(stream);
            }

            Err(last_error.unwrap_or_else(|| ProviderError::Other("max retries exceeded".into())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ContentBlock, Message, Role};
    use crate::providers::ToolDef;
    use futures::StreamExt;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn streams_text_from_openai_compatible_endpoint() {
        let server = MockServer::start().await;

        let sse_body = "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\ndata: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("Authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse_body))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new("test-key".into()).with_base_url(server.uri());

        let stream = provider
            .chat(
                vec![Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text { text: "hi".into() }],
                }],
                vec![],
            )
            .await
            .unwrap();

        let blocks: Vec<ContentBlock> = stream.collect().await;

        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[0],
            ContentBlock::Text {
                text: "Hello".into()
            }
        );
        assert_eq!(
            blocks[1],
            ContentBlock::Text {
                text: " world".into()
            }
        );
    }

    #[tokio::test]
    async fn assembles_tool_call_deltas() {
        let server = MockServer::start().await;

        let sse_body = "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"bash\",\"arguments\":\"\"}}]}}]}\n\ndata: {\"id\":\"1\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"command\\\":\"}}]}}]}\n\ndata: {\"id\":\"1\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"ls\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse_body))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new("test-key".into()).with_base_url(server.uri());

        let stream = provider
            .chat(
                vec![Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: "list files".into(),
                    }],
                }],
                vec![ToolDef {
                    name: "bash".into(),
                    description: "run command".into(),
                    parameters: serde_json::json!({}),
                }],
            )
            .await
            .unwrap();

        let blocks: Vec<ContentBlock> = stream.collect().await;

        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            ContentBlock::ToolCall {
                id,
                name,
                arguments,
            } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "bash");
                assert_eq!(arguments, "{\"command\":\"ls\"}");
            }
            other => panic!("expected ToolCall, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn retries_on_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .expect(2)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new("test-key".into()).with_base_url(server.uri());

        let stream = provider
            .chat(
                vec![Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text { text: "hi".into() }],
                }],
                vec![],
            )
            .await
            .unwrap();

        let blocks: Vec<ContentBlock> = stream.collect().await;
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn missing_api_key_error_is_clear() {
        let provider = OpenAiProvider::new("".into());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(provider.chat(
            vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text { text: "hi".into() }],
            }],
            vec![],
        ));
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("API key") || msg.contains("401"),
                    "expected auth error, got: {}",
                    msg
                );
            }
            Ok(_) => panic!("expected error"),
        }
    }
}
