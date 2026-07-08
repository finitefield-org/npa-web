# AGENTS.md

Working guidelines for agents operating in this repository.

## Repository-wide Guidelines

- Use `/usr/bin/git` for git commands.
- Do not use Git LFS in this repository.
- Do not add `.gitattributes` rules that set `filter=lfs`, `diff=lfs`, or
  `merge=lfs`.
- When importing or updating subtree content, ensure LFS pointer files are not
  introduced into this repository history.
- Record any suggestions in `suggestions.md`.
- When adding proofs, make maximum effort to choose module and theorem names
  that accurately communicate their mathematical meaning and corpus role.
- When module or theorem refactoring is needed to preserve meaningful naming,
  semantic placement, or maintainable proof organization, perform it without
  hesitation rather than leaving proofs in ill-fitting modules or names.
