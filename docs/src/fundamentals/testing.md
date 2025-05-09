# Testing and Quality Assurance

We follow the Testing Pyramid approach to testing,
the majority of tests are unit tests,
with a smaller number of integration tests,
and a few end-to-end tests.

Unit and integration tests are written in Rust,
end-to-end tests are written either in Rust or in JavaScript using Playwright.

> [!note]
> We still need to verify that playwright works well with the current setup.
>
> Playwright may be used for the following:
> - End-to-end tests simulating *entire user flows*.
> - Visual regression tests.
>
> For single user interactions, we should not need Playwright.
> For these, we should use the Rust testing framework.

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
