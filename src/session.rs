use crate::protocol::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub profile: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn save_session(
    session_id: &str,
    profile: &str,
    messages: &[Message],
    dir: &std::path::Path,
) -> Result<(), SessionError> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.jsonl", session_id));
    let now = chrono::Utc::now().to_rfc3339();
    let meta = SessionMeta {
        session_id: session_id.to_string(),
        profile: profile.to_string(),
        created: now.clone(),
        updated: now,
    };
    let mut content = serde_json::to_string(&meta)?;
    content.push('\n');
    for msg in messages {
        content.push_str(&serde_json::to_string(msg)?);
        content.push('\n');
    }
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn load_session(
    session_id: &str,
    dir: &std::path::Path,
) -> Result<(SessionMeta, Vec<Message>), SessionError> {
    let path = dir.join(format!("{}.jsonl", session_id));
    if !path.exists() {
        return Err(SessionError::NotFound(session_id.to_string()));
    }
    let content = std::fs::read_to_string(&path)?;
    let mut messages = vec![];
    let mut meta: Option<SessionMeta> = None;
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        if meta.is_none() {
            meta = Some(serde_json::from_str(line)?);
        } else {
            messages.push(serde_json::from_str(line)?);
        }
    }
    match meta {
        Some(m) => Ok((m, messages)),
        None => Err(SessionError::NotFound(session_id.to_string())),
    }
}

pub fn list_sessions(dir: &std::path::Path) -> Result<Vec<SessionMeta>, SessionError> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut sessions = vec![];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            if let Some(first_line) = content.lines().next() {
                if let Ok(meta) = serde_json::from_str::<SessionMeta>(first_line) {
                    sessions.push(meta);
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.updated.cmp(&a.updated));
    Ok(sessions)
}

pub fn default_sessions_dir() -> std::path::PathBuf {
    crate::profile::default_profiles_dir()
        .parent()
        .unwrap()
        .join("sessions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ContentBlock, Role};

    #[test]
    fn save_and_load_session_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let messages = vec![
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "hello".into(),
                }],
            },
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "hi there".into(),
                }],
            },
        ];

        save_session("test-session", "coder", &messages, dir.path()).unwrap();

        let (meta, loaded) = load_session("test-session", dir.path()).unwrap();
        assert_eq!(meta.profile, "coder");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, Role::User);
        assert_eq!(loaded[1].role, Role::Assistant);
    }

    #[test]
    fn list_sessions_finds_saved() {
        let dir = tempfile::tempdir().unwrap();
        let messages = vec![];
        save_session("s1", "coder", &messages, dir.path()).unwrap();
        save_session("s2", "researcher", &messages, dir.path()).unwrap();

        let sessions = list_sessions(dir.path()).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn load_nonexistent_session() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_session("nope", dir.path());
        assert!(result.is_err());
    }
}
