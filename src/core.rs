use crate::context;
use crate::engine;
use crate::profile::{self, Profile};
use crate::protocol::{ContentBlock, Message, Role};
use crate::providers::{OpenAiProvider, StubProvider};
use crate::skills;
use crate::tools::{BashTool, ToolRegistry};

pub fn resolve_api_key(args: &crate::cli::Args) -> Option<String> {
    resolve_api_key_with(args, |key| std::env::var(key).ok())
}

pub fn resolve_api_key_with(
    args: &crate::cli::Args,
    env_lookup: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    args.api_key
        .clone()
        .or_else(|| env_lookup("OPENAI_API_KEY"))
}

pub async fn run(args: crate::cli::Args) {
    let api_key = resolve_api_key(&args);

    let profile = match &args.profile {
        Some(name) => match profile::load_profile(name, &profile::default_profiles_dir()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error loading profile '{}': {}", name, e);
                std::process::exit(1);
            }
        },
        None => {
            if api_key.is_some() {
                Profile {
                    name: "default".into(),
                    provider: "openai".into(),
                    model: "gpt-4o".into(),
                    system_prompt: "You are a helpful AI assistant.".into(),
                    skills: vec![],
                    tools: vec!["bash".to_string()],
                    max_iterations: 50,
                    max_tokens: None,
                }
            } else {
                eprintln!("Set OPENAI_API_KEY or use --api-key, or specify --profile.");
                std::process::exit(1);
            }
        }
    };

    let api_key = api_key.unwrap_or_default();

    let mut registry = ToolRegistry::new();
    for tool_name in &profile.tools {
        match tool_name.as_str() {
            "bash" => registry.register(BashTool),
            other => eprintln!("Warning: unknown tool '{}' in profile", other),
        }
    }

    let loaded_skills = match skills::load_skills(&skills::default_skills_dir()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: could not load skills: {}", e);
            vec![]
        }
    };

    let selected_skills: Vec<_> = loaded_skills
        .into_iter()
        .filter(|s| profile.skills.contains(&s.name))
        .collect();

    let system_prompt = skills::inject_skills_into_prompt(&profile.system_prompt, &selected_skills);

    let messages = if let Some(session_id) = &args.session {
        match crate::session::load_session(session_id, &crate::session::default_sessions_dir()) {
            Ok((meta, msgs)) => {
                eprintln!("Resumed session {} (profile: {})", session_id, meta.profile);
                msgs
            }
            Err(e) => {
                eprintln!("Error loading session '{}': {}", session_id, e);
                std::process::exit(1);
            }
        }
    } else {
        vec![Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: system_prompt,
            }],
        }]
    };

    if messages.iter().all(|m| m.role != Role::User) {
        println!("Praxis ready. Type your message:");
    }

    let result = if profile.provider == "openai" {
        if api_key.is_empty() {
            eprintln!("Error: OPENAI_API_KEY not set");
            std::process::exit(1);
        }
        let provider = OpenAiProvider::new(api_key);
        let tools = registry.definitions();
        let max_tokens = profile.max_tokens.unwrap_or(120_000);
        let _trimmed = context::trim_messages(messages.clone(), max_tokens);
        engine::run_with_tools(
            &provider,
            _trimmed,
            tools,
            &registry,
            profile.max_iterations,
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Provider error: {}", e);
            std::process::exit(1);
        })
    } else {
        let provider = StubProvider;
        engine::run_with_tools(
            &provider,
            messages,
            vec![],
            &registry,
            profile.max_iterations,
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Provider error: {}", e);
            std::process::exit(1);
        })
    };

    for msg in &result {
        if msg.role == Role::Assistant {
            for block in &msg.content {
                match block {
                    ContentBlock::Text { text } => println!("{}", text),
                    ContentBlock::ToolCall {
                        name, arguments, ..
                    } => {
                        println!("🔧 Running {}: {}", name, arguments);
                    }
                    ContentBlock::ToolResult { name, content, .. } => {
                        println!("📋 {} result: {}", name, content);
                    }
                }
            }
        }
    }

    if args.save {
        let session_id = args
            .session
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string());
        if let Err(e) = crate::session::save_session(
            &session_id,
            &profile.name,
            &result,
            &crate::session::default_sessions_dir(),
        ) {
            eprintln!("Error saving session: {}", e);
        } else {
            eprintln!("Session saved as: {}", session_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_from_cli_arg() {
        let args = crate::cli::Args {
            profile: None,
            session: None,
            save: false,
            list_sessions: false,
            api_key: Some("sk-cli".into()),
        };
        assert_eq!(resolve_api_key(&args), Some("sk-cli".into()));
    }

    #[test]
    fn api_key_from_env_var_fallback() {
        let args = crate::cli::Args {
            profile: None,
            session: None,
            save: false,
            list_sessions: false,
            api_key: None,
        };
        let env = |key: &str| {
            if key == "OPENAI_API_KEY" {
                Some("sk-env".into())
            } else {
                None
            }
        };
        assert_eq!(resolve_api_key_with(&args, env), Some("sk-env".into()));
    }

    #[test]
    fn api_key_missing_both() {
        let args = crate::cli::Args {
            profile: None,
            session: None,
            save: false,
            list_sessions: false,
            api_key: None,
        };
        assert_eq!(resolve_api_key_with(&args, |_| None), None);
    }

    #[test]
    fn cli_arg_takes_priority_over_env() {
        let args = crate::cli::Args {
            profile: None,
            session: None,
            save: false,
            list_sessions: false,
            api_key: Some("sk-cli".into()),
        };
        let env = |key: &str| {
            if key == "OPENAI_API_KEY" {
                Some("sk-env".into())
            } else {
                None
            }
        };
        assert_eq!(resolve_api_key_with(&args, env), Some("sk-cli".into()));
    }
}
