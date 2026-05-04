use std::pin::Pin;

use crate::protocol::{ContentBlock, Message, ToolDef};
use futures::Stream;

pub mod openai;

pub type ChatStream = Pin<Box<dyn Stream<Item = ContentBlock> + Send>>;

#[allow(clippy::manual_async_fn)]
pub trait Provider {
    fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
    ) -> impl std::future::Future<Output = Result<ChatStream, ProviderError>> + Send;
}

pub use openai::OpenAiProvider;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider error: {0}")]
    Other(String),
}

pub struct StubProvider;

#[allow(clippy::manual_async_fn)]
impl Provider for StubProvider {
    fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolDef>,
    ) -> impl std::future::Future<Output = Result<ChatStream, ProviderError>> + Send {
        async {
            let stream: ChatStream = Box::pin(futures::stream::once(async {
                ContentBlock::Text {
                    text: "Hello from Praxis!".into(),
                }
            }));
            Ok(stream)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn stub_provider_returns_greeting() {
        let provider = StubProvider;
        let mut stream = provider
            .chat(
                vec![Message {
                    role: crate::protocol::Role::User,
                    content: vec![ContentBlock::Text { text: "hi".into() }],
                }],
                vec![],
            )
            .await
            .unwrap();

        let mut blocks = vec![];
        while let Some(block) = stream.next().await {
            blocks.push(block);
        }

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0],
            ContentBlock::Text {
                text: "Hello from Praxis!".into(),
            }
        );
    }
}
