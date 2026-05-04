use clap::Parser;

#[derive(Parser)]
#[command(name = "praxis", version, about = "Personal AI agent framework")]
pub struct Args {
    #[arg(short, long, help = "Profile to use")]
    pub profile: Option<String>,

    #[arg(long, help = "Session ID to resume")]
    pub session: Option<String>,

    #[arg(long, help = "Save session on exit")]
    pub save: bool,

    #[arg(long, help = "List saved sessions")]
    pub list_sessions: bool,

    #[arg(long, help = "OpenAI API key (or set OPENAI_API_KEY env var)")]
    pub api_key: Option<String>,
}

pub fn parse() -> Args {
    Args::parse()
}

pub async fn run(args: Args) {
    if args.list_sessions {
        let dir = crate::session::default_sessions_dir();
        match crate::session::list_sessions(&dir) {
            Ok(sessions) => {
                if sessions.is_empty() {
                    println!("No saved sessions.");
                } else {
                    for s in &sessions {
                        println!("  {}  {}  {}", s.session_id, s.profile, s.updated);
                    }
                }
            }
            Err(e) => eprintln!("Error listing sessions: {}", e),
        }
        return;
    }

    crate::core::run(args).await;
}
