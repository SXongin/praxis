use crate::protocol::{ContentBlock, Message, Role};

pub fn estimate_tokens(messages: &[Message]) -> usize {
    let mut total = 0;
    for msg in messages {
        for block in &msg.content {
            match block {
                ContentBlock::Text { text } => {
                    total += text.len() / 4 + 1;
                }
                ContentBlock::ToolCall { id, name, arguments } => {
                    total += (id.len() + name.len() + arguments.len()) / 4 + 3;
                }
                ContentBlock::ToolResult {
                    tool_call_id,
                    name,
                    content,
                } => {
                    total += (tool_call_id.len() + name.len() + content.len()) / 4 + 3;
                }
            }
        }
    }
    total
}

pub fn trim_messages(mut messages: Vec<Message>, max_tokens: usize) -> Vec<Message> {
    while estimate_tokens(&messages) > max_tokens && messages.len() > 1 {
        let first_non_system = messages
            .iter()
            .position(|m| m.role != Role::System)
            .unwrap_or(0);
        messages.remove(first_non_system);
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ContentBlock, Message, Role};

    #[test]
    fn estimate_counts_all_text() {
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "hello world".into(),
            }],
        }];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn trim_drops_oldest_non_system() {
        let messages = vec![
            Message {
                role: Role::System,
                content: vec![ContentBlock::Text {
                    text: "system prompt".into(),
                }],
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "old message".into(),
                }],
            },
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "old reply".into(),
                }],
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "new message".into(),
                }],
            },
        ];

        let max = estimate_tokens(&messages) - 3;
        let trimmed = trim_messages(messages, max);
        assert_eq!(trimmed[0].role, Role::System);
        assert!(trimmed.len() < 4);
    }

    #[test]
    fn trim_preserves_system_message() {
        let messages = vec![
            Message {
                role: Role::System,
                content: vec![ContentBlock::Text {
                    text: "very important system prompt".into(),
                }],
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "user message".into(),
                }],
            },
        ];

        let trimmed = trim_messages(messages.clone(), 1);
        assert_eq!(trimmed.len(), 1);
        assert_eq!(trimmed[0].role, Role::System);
    }
}
