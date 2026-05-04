use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("skill parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn load_skills(dir: &std::path::Path) -> Result<Vec<Skill>, SkillError> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut skills = vec![];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                let skill = load_skill(&skill_md)?;
                skills.push(skill);
            }
        }
    }
    Ok(skills)
}

pub fn load_skill(path: &std::path::Path) -> Result<Skill, SkillError> {
    let content = std::fs::read_to_string(path)?;
    parse_skill(&content)
}

pub fn parse_skill(content: &str) -> Result<Skill, SkillError> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err(SkillError::Parse("missing YAML frontmatter".into()));
    }
    let rest = &content[3..];
    let end = rest
        .find("---")
        .ok_or_else(|| SkillError::Parse("unclosed YAML frontmatter".into()))?;
    let frontmatter = &rest[..end].trim();
    let body = rest[end + 3..].trim().to_string();

    #[derive(Deserialize)]
    struct Frontmatter {
        name: String,
        #[serde(default)]
        description: Option<String>,
    }

    let fm: Frontmatter =
        serde_yaml::from_str(frontmatter).map_err(|e| SkillError::Parse(e.to_string()))?;

    Ok(Skill {
        name: fm.name,
        description: fm.description,
        body,
    })
}

pub fn inject_skills_into_prompt(
    system_prompt: &str,
    skills: &[Skill],
) -> String {
    if skills.is_empty() {
        return system_prompt.to_string();
    }
    let mut prompt = system_prompt.to_string();
    prompt.push_str("\n\n## Skills\n\n");
    for skill in skills {
        prompt.push_str(&format!("### {}\n", skill.name));
        prompt.push_str(&skill.body);
        prompt.push_str("\n\n");
    }
    prompt
}

pub fn default_skills_dir() -> std::path::PathBuf {
    crate::profile::default_profiles_dir()
        .parent()
        .unwrap()
        .join("skills")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_skill_md() {
        let content = "---\nname: test-skill\ndescription: A test\n---\n\nThis is the body.";
        let skill = parse_skill(content).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description.as_deref(), Some("A test"));
        assert_eq!(skill.body, "This is the body.");
    }

    #[test]
    fn parse_minimal_skill() {
        let content = "---\nname: minimal\n---\nbody here";
        let skill = parse_skill(content).unwrap();
        assert_eq!(skill.name, "minimal");
        assert!(skill.description.is_none());
        assert_eq!(skill.body, "body here");
    }

    #[test]
    fn parse_missing_frontmatter() {
        let result = parse_skill("no frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn inject_skills_adds_body_to_prompt() {
        let skills = vec![Skill {
            name: "test".into(),
            description: None,
            body: "do the thing".into(),
        }];
        let prompt = inject_skills_into_prompt("base prompt", &skills);
        assert!(prompt.contains("base prompt"));
        assert!(prompt.contains("## Skills"));
        assert!(prompt.contains("### test"));
        assert!(prompt.contains("do the thing"));
    }
}
