# Branch Conventions

## Model

GitHub Flow: all work on feature branches, merge to `main` via PR.

## Branch Naming

| Prefix | Use | Example |
|---|---|---|
| `feat/` | New feature | `feat/add-anthropic-provider` |
| `fix/` | Bug fix | `fix/session-resume-crash` |
| `refactor/` | Refactor | `refactor/engine-error-handling` |
| `docs/` | Documentation | `docs/update-context-md` |
| `chore/` | Maintenance | `chore/update-deps` |

Human and AI share the same naming convention.

## Merge Policy

All PRs require human review and approval before merge. No auto-merge. Merge via rebase only (linear history).

## Commit Messages

Format: `type(scope): description`

| Type | Use |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code restructuring without behavior change |
| `docs` | Documentation only |
| `chore` | Maintenance (deps, CI, config) |
| `style` | Formatting, whitespace (no logic change) |
| `test` | Adding or fixing tests |
| `ci` | CI/CD changes |
| `perf` | Performance improvement |

Rules:
- Subject line ≤ 72 characters
- Lowercase after colon
- No period at end
- Use imperative mood ("add" not "added")

Example: `feat(providers): add Anthropic provider support`

Use `/clean-commits` command to squash and rewrite commits before merge.

Enforced by commitlint (`commitlint.config.js`):
- CI: `opensource-nepal/commitlint@v1` checks all PR commits
- Local: `.githooks/commit-msg` hook validates before commit

## Push After Rebase

After rebasing or amending commits on a feature branch, use `--force-with-lease` instead of `--force`:

```bash
git push --force-with-lease
```

This prevents overwriting remote changes if someone else pushed to the same branch. Safer than `--force`.

## CI Gate

No PR can merge to `main` unless:
- `cargo fmt --check` passes
- `cargo clippy -D warnings` passes
- `cargo doc --no-deps` passes
- `cargo build` succeeds
- `cargo test` passes
- `cargo deny check` passes
- All commit messages pass commitlint
- All 3 OS matrix (ubuntu, windows, macos) pass

## Git Hooks

Setup: `git config core.hooksPath .githooks`

- `.githooks/pre-commit` — runs `cargo fmt --check` + `cargo clippy` on staged `.rs` files
- `.githooks/commit-msg` — runs commitlint to validate commit message format

## Workflows

- `.github/workflows/ci.yml` — CI on push/PR to main

## Branch Protection (GitHub Rulesets)

Configure in repo Settings > Rulesets:
- Require pull request before merging
- Require status checks to pass: `build (ubuntu-latest)`, `build (windows-latest)`, `build (macos-latest)`, `deny`, `commits`
- Require branches to be up to date before merging
- Do not allow bypassing the above settings
