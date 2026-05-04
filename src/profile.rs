use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default)]
    pub max_tokens: Option<usize>,
}

fn default_max_iterations() -> usize {
    50
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("profile not found: {0}")]
    NotFound(String),
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn load_profile(name: &str, dir: &std::path::Path) -> Result<Profile, ProfileError> {
    let path = dir.join(format!("{}.yaml", name));
    if !path.exists() {
        return Err(ProfileError::NotFound(name.to_string()));
    }
    let content = std::fs::read_to_string(&path)?;
    let profile: Profile =
        serde_yaml::from_str(&content).map_err(|e| ProfileError::Invalid(e.to_string()))?;
    if profile.name != name {
        return Err(ProfileError::Invalid(format!(
            "profile name '{}' does not match filename '{}'",
            profile.name, name
        )));
    }
    Ok(profile)
}

pub fn list_profiles(dir: &std::path::Path) -> Result<Vec<String>, ProfileError> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = vec![];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "yaml").unwrap_or(false)
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub fn default_profiles_dir() -> std::path::PathBuf {
    dirs_config().join("praxis").join("profiles")
}

fn dirs_config() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(dir)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        std::path::PathBuf::from(home).join(".config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_valid_profile() {
        let yaml = r#"
name: coder
provider: openai
model: gpt-4o
system_prompt: "You are a coder."
skills:
  - code-reviewer
tools:
  - bash
"#;
        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.name, "coder");
        assert_eq!(profile.provider, "openai");
        assert_eq!(profile.skills, vec!["code-reviewer"]);
        assert_eq!(profile.tools, vec!["bash"]);
        assert_eq!(profile.max_iterations, 50);
    }

    #[test]
    fn parse_profile_with_defaults() {
        let yaml = r#"
name: minimal
provider: openai
model: gpt-4o
system_prompt: "hi"
"#;
        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert!(profile.skills.is_empty());
        assert!(profile.tools.is_empty());
        assert_eq!(profile.max_iterations, 50);
    }

    #[test]
    fn load_profile_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "name: test\nprovider: openai\nmodel: gpt-4o\nsystem_prompt: hello"
        )
        .unwrap();

        let profile = load_profile("test", dir.path()).unwrap();
        assert_eq!(profile.name, "test");
    }

    #[test]
    fn load_nonexistent_profile() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_profile("nonexistent", dir.path());
        assert!(result.is_err());
    }
}
