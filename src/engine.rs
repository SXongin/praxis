use crate::protocol::ToolDef;
use crate::protocol::{ContentBlock, Message, Role};
use crate::providers::Provider;
use crate::providers::ProviderError;

pub async fn run_with_tools<P: Provider>(
    provider: &P,
    messages: Vec<Message>,
    tools: Vec<ToolDef>,
    registry: &crate::tools::ToolRegistry,
    max_iterations: usize,
) -> Result<Vec<Message>, ProviderError> {
    let mut messages = messages;
    let mut iterations = 0;

    loop {
        iterations += 1;
        if iterations > max_iterations {
            messages.push(Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "Reached maximum iterations.".into(),
                }],
            });
            break;
        }

        let mut stream = provider.chat(messages.clone(), tools.clone()).await?;
        let mut blocks = vec![];
        use futures::StreamExt;
        while let Some(block) = stream.next().await {
            blocks.push(block);
        }

        messages.push(Message {
            role: Role::Assistant,
            content: blocks.clone(),
        });

        let tool_calls: Vec<_> = blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolCall {
                    id,
                    name,
                    arguments,
                } => Some((id.clone(), name.clone(), arguments.clone())),
                _ => None,
            })
            .collect();

        if tool_calls.is_empty() {
            break;
        }

        for (id, name, args) in tool_calls {
            let args_value: serde_json::Value =
                serde_json::from_str(&args).unwrap_or(serde_json::json!({}));
            let result = match registry.execute(&name, args_value).await {
                Ok(output) => output,
                Err(e) => e.to_string(),
            };
            messages.push(Message {
                role: Role::Tool,
                content: vec![ContentBlock::ToolResult {
                    tool_call_id: id,
                    name,
                    content: result,
                }],
            });
        }
    }

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ContentBlock, Message, Role};
    use crate::providers::StubProvider;
    use crate::tools::{BashTool, ToolRegistry};

    #[tokio::test]
    async fn engine_returns_messages_with_assistant_response() {
        let provider = StubProvider;
        let input = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "hello".into(),
            }],
        }];

        let registry = ToolRegistry::new();
        let result = run_with_tools(&provider, input, vec![], &registry, 10)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[1].role, Role::Assistant);
        assert_eq!(
            result[1].content[0],
            ContentBlock::Text {
                text: "Hello from Praxis!".into(),
            }
        );
    }

    #[tokio::test]
    async fn multi_turn_executes_tool_and_returns_result() {
        use crate::protocol::{ContentBlock, Message, Role};
        use crate::providers::{Provider, ProviderError};

        struct ToolCallProvider;

        impl Provider for ToolCallProvider {
            fn chat(
                &self,
                messages: Vec<Message>,
                _tools: Vec<ToolDef>,
            ) -> impl std::future::Future<
                Output = Result<crate::providers::ChatStream, ProviderError>,
            > + Send {
                async move {
                    let last_role = messages.last().map(|m| &m.role);
                    let blocks = match last_role {
                        Some(Role::Tool) => vec![ContentBlock::Text {
                            text: "command completed successfully".into(),
                        }],
                        _ => vec![ContentBlock::ToolCall {
                            id: "call_1".into(),
                            name: "bash".into(),
                            arguments: r#"{"command":"echo hello"}"#.into(),
                        }],
                    };
                    let stream: crate::providers::ChatStream =
                        Box::pin(futures::stream::iter(blocks));
                    Ok(stream)
                }
            }
        }

        let provider = ToolCallProvider;
        let mut registry = ToolRegistry::new();
        registry.register(BashTool);

        let input = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "run echo".into(),
            }],
        }];

        let result = run_with_tools(&provider, input, vec![], &registry, 10)
            .await
            .unwrap();

        // User → Assistant(tool_call) → Tool(result) → Assistant(text)
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[1].role, Role::Assistant);
        assert_eq!(result[2].role, Role::Tool);
        assert_eq!(result[3].role, Role::Assistant);
        assert!(matches!(
            result[1].content[0],
            ContentBlock::ToolCall { .. }
        ));
        assert!(matches!(
            result[2].content[0],
            ContentBlock::ToolResult { .. }
        ));
        assert_eq!(
            result[3].content[0],
            ContentBlock::Text {
                text: "command completed successfully".into(),
            }
        );
    }
}
