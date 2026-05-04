# Skills and Tools are orthogonal axes

**Context:** The PRD proposed a two-dimension capability model — Skills (knowledge, SKILL.md) and Tools (capability, function-calling). The question was whether a skill could declare or bundle tools, making skills the delivery mechanism for both knowledge and capabilities.

**Decision:** In V1, Skills and Tools are orthogonal axes. A skill is knowledge-only (SKILL.md injected into the model's context). A tool is a compiled Rust struct implementing the Tool trait. A profile selects skills and tools independently. No skill can declare, bundle, or require a tool.

**Why:**

- **Tools are compiled-in, Skills are external.** Tools are Rust code in the binary. Skills live on disk. Coupling them means either embedding tool references in skill files (fragile) or making skills Rust-code-aware (defeats the purpose of SKILL.md being a plain markdown file).
- **Simpler mental model.** "Skills tell the model what to think, Tools give the model what to do." No overlap, no confusion.
- **The use case hasn't emerged.** We couldn't identify a concrete V1 scenario where a skill genuinely needs a dedicated tool beyond the universal tools (bash, read, write) that every profile includes.
- **Easy to revisit.** If later we find skills truly need bundled tools, we can add an optional `requires_tools` field in SKILL.md frontmatter. Nothing in the V1 architecture prevents this.

**Considered alternative:** Skills that declare tool dependencies in their frontmatter, with the profile resolver ensuring those tools are registered. Rejected because it adds coupling without a demonstrated V1 need.
