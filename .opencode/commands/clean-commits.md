---
description: Clean up commit history before merge
---

Review the commit history on this branch since it diverged from main.

1. Run `git log --oneline main..HEAD` to see all commits
2. Identify commits that should be squashed together (e.g., multiple fix/WIP commits for the same feature)
3. Identify commits with non-conventional messages that need rewriting
4. Perform an interactive rebase to:
   - Squash related commits into logical units
   - Rewrite messages to follow conventional commits format: `type(scope): description`
   - Types: feat, fix, refactor, docs, chore, style, test, ci, perf
5. Show the final clean history and confirm with the user
6. Force push the cleaned branch

Focus on making each commit a meaningful, self-contained change that makes sense in the project history.
