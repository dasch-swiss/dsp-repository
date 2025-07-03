# Testing and Quality Assurance

We follow the Testing Pyramid approach to testing,
the majority of tests are unit tests,
with a smaller number of integration tests,
and a few end-to-end tests.

Unit and integration tests are written in Rust,
end-to-end tests are written either in Rust or in JavaScript using Playwright.

## Playwright Setup

Playwright is set up for design system component testing with two approaches:

**Interactive Testing (MCP)**:
- Start playground server: `just run-watch-playground`
- Use Claude Code with Playwright MCP commands for visual verification
- Commands whitelisted in `.claude/settings.json`
- Best for: Component development, design verification, manual testing

**Automated Testing (CI/CD)**:
- TypeScript-based Playwright setup with comprehensive tooling
- Functional, accessibility, and responsive design testing in CI
- Visual regression testing disabled in CI (runs locally only)
- ESLint, Prettier, and strict TypeScript checking
- HTML + JSON reporters for CI/CD integration
- Video recording and screenshots on test failures
- Best for: End-to-end user flows, automated regression detection

Setup: `just playground install` then `just playground test`

Available commands:
- `just playground test-ui` - Interactive test runner
- `just playground test-debug` - Debug mode with browser DevTools
- `just playground docker-update-visuals` - Update visual baselines (Linux-consistent)
- `just playground update-visuals` - Update visual baselines (platform-specific)
- `just playground docker-test` - Run tests in Docker (matches CI environment)
- `just playground docker-build` - Build Docker image for testing
- `just playground type-check` - TypeScript validation
- `just playground lint-and-format` - Code quality checks

**Visual Testing (Local Development Only)**:
Visual regression tests are disabled in CI to avoid cross-platform font rendering differences. For local visual validation:
1. `just playground test` - Runs all tests including visual regression
2. `just playground docker-update-visuals` - Update visual baselines using consistent Docker environment
3. `just playground docker-test` - Test using Docker environment

**Note**: CI runs functional, accessibility, and responsive tests only. Visual validation should be performed locally during development.

For single component interactions, prefer Rust tests. Playwright is for visual verification and complete user flows.

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
