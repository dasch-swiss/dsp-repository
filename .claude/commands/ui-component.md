---
description: Create a new UI component in the design system. Provide Tailwind HTML for reference.
argument-hint: Component Name | Additional details (optional)
---

## Context

If provided, parse the command arguments as follows:

- [name]: $1
- [details] - optional: $2

If this does not work, try to make sense of $ARGUMENTS as best as you can.

You should now have a component name, and optional details specified.
If not, ask the user for clarification.

## Your Task

Create a new UI component for our design system based on the provided component name and any additional details.

- Create the component in @modules/design_system/components/
- Follow the design system's coding standards and architecture
- Ensure the component is accessible and responsive
- Write comprehensive tests for the component, including unit, component tests (integration) and E2E tests
- Add the component to the design system playground for demonstration and testing
- If applicable, use the component in example usage pages
- Document the component's usage, props, and any relevant details in @docs/src/design_system/components/

## What to Build

The DSP design system is built on top of Tailwind components. 
Use the Tailwind reference files as a guide for structure, styling, and behavior.

The design system's component library is built with Rust using the Maud templating engine for server-side rendering. 
It is meant to create multi-page web applications with minimal client-side state. 
It follows Clean Architecture principles and emphasizes accessibility, responsiveness, and code quality. 
Use the existing components in @modules/design_system/components/ as examples of structure and style.

## How to Proceed

To implement the component, follow these steps:

- Analyze the component requirements based on the name and details provided. If anything is unclear, ask for clarification.
- Plan the component structure, props, and any necessary state or interactions. 
  Confirm this plan with the developer before proceeding.
- Check the Tailwind reference file. 
  Unless specified otherwise, assume that the reference file is located in the @.claude/tailwind/ directory.
  If you cannot find it, ask the developer for the correct location.
- Implement the component, following the reference closely.
  Use the exact same utility classes for styling, unless necessitated otherwise.
  Follow the reference's HTML structure exactly, adapting only as needed for the required functionality.
  If multiple references exist, clarify with the developer which one to follow. 
  If you are asked to follow multiple references, combine them logically and according to the functionality required.
- Task orchestration
  - For implementation, use the "design-system-component-builder" sub-agent.
  - For verification, use the "design-system-qa" sub-agent.
  - If the component is not up to standard according to the "design-system-qa" sub-agent,
    iterate with the "design-system-component-builder" sub-agent until it meets the requirements.
  - When finished, use the "design-system-docs" sub-agent to create or improve documentation for the component.

If any major issues arise during implementation, such as the need for fundamental architectural changes,
ask the developer for guidance before proceeding.
