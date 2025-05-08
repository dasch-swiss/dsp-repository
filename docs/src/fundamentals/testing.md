# Testing and Quality Assurance

We follow the Testing Pyramid approach to testing,
the majority of tests are unit tests,
with a smaller number of integration tests,
and a few end-to-end tests.

Unit and integration tests are written in Rust,
end-to-end tests are written in JavaScript using Playwright.

> [!note]
> We still need to verify that playwright works well with the current setup.

We aim to follow the practice of Test Driven Development (TDD),
where tests are written before the code is implemented.
This helps to ensure that the code is testable and meets the requirements.
