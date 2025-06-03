# Workflows and Conventions

## Entry Points

The first entry point of this repository is the README file,
which should give anyone an indication of where to find any information they need.

For any interaction or coding-related workflow, the justfile is the primary source of truth.
The justfile contains all the commands and workflows that are used in this repository, along with their descriptions.

Any further information should be located in the documentation.

## Git Workflow

For this repository, we use a rebase workflow.
This means that all changes should be made on a branch,
and then rebased onto the main branch before being merged.

This allows us to keep a clean commit history and avoid merge commits.
