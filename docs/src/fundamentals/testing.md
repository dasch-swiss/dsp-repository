# Testing and Quality Assurance

We follow the Testing Pyramid approach to testing,
the majority of tests are unit tests,
with a smaller number of integration tests,
and a few end-to-end tests.

Unit and integration tests are written in Rust,
end-to-end tests are written either in Rust or in JavaScript using Playwright.

## Design System Testing

The design system playground includes comprehensive testing infrastructure:

**Interactive Testing (MCP)**:
- Start playground server: `just run-watch-playground`
- Use Claude Code with Playwright MCP commands for visual verification
- Commands whitelisted in `.claude/settings.json`
- Best for: Component development, design verification, manual testing

**Automated Testing (CI/CD)**:
- TypeScript-based Playwright setup with tooling
- Functional, accessibility, and responsive design testing in CI
- ESLint, Prettier, and TypeScript checking
- HTML + JSON reporters for CI/CD integration
- Best for: End-to-end user flows, automated regression detection

**Setup**: `just playground install` then `just playground test`

**Key Commands**:
- `just playground test` - Run all tests
- `just playground test-ui` - Interactive test runner
- `just playground test-debug` - Debug mode with browser DevTools
- `just playground type-check` - TypeScript validation
- `just playground lint-and-format` - Code quality checks

For single component interactions, prefer Rust tests. Playwright is for complete user flows.

> [!note]
> We are still evaluating Playwright integration for broader testing use cases.

Unit tests are the foundation of our testing strategy.
They test individual components in isolation,
ensuring that each part of the codebase behaves as expected.
Unit tests are fast to write and to execute,
and they provide immediate feedback on the correctness of the code.

Integration tests verify the interaction between different components,
ensuring that they work together as expected.
Integration tests may check the integration between the business logic and the presentation layer,
or between the view and the business logic.

End-to-end tests verify the entire system.
They simulate real user interactions and check that the system behaves as expected.

Additional to the functional tests,
we also need to implement performance tests.

We aim to follow the practice of Test Driven Development (TDD),
where tests are written before the code is implemented.
This helps to ensure that the code is testable and meets the requirements.
