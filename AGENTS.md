## Agent skills

### Issue tracker

Issues live as markdown files under `.scratch/` in this repo. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: one `CONTEXT.md` + `docs/adr/` at repo root. See `docs/agents/domain.md`.

### Branch conventions

GitHub Flow with tiered merge policy. See `docs/agents/branch-conventions.md`.

### Pre-commit hook

A git pre-commit hook runs `cargo fmt --check` and `cargo clippy -D warnings` on staged `.rs` files. If commit fails:

1. `cargo fmt` failures: files are auto-fixed. Run `cargo fmt --all`, `git add -u`, then commit again.
2. `cargo clippy` failures: read the error output, fix the issues, then commit again.
3. Never use `--no-verify` to bypass the hook.
