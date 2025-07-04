# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the DSP Design System Playground.

## Purpose

Live development environment for testing and developing design system components in isolation.

## Server Details

- **Port**: 3400 (http://localhost:3400)
- **Live Reload**: Automatic browser refresh via WebSocket when files change
- **Static Assets**: Served from `/assets` directory

## Development Workflow

- **Start Server**: `just run-watch-playground`
- **Visual Testing**: Use WebFetch tool to view rendered components at `http://localhost:3400/[component]` while developing
- **Page Structure**: Each component gets its own dedicated route/page for testing
- **Component Routes**:
  - `/` - Home page with component list
  - `/button` - Button component examples
  - `/banner` - Banner component examples
  - `/shell` - Shell component examples

## Visual Testing Notes

Visual regression tests are platform-specific and generate different baselines on different operating systems:

- **Platform Dependency**: Visual baselines should be generated on the same OS where tests will run
- **Local Testing**: Use `just update-visuals` to generate baselines for your platform
- **CI Considerations**: Visual tests may need to be disabled or run conditionally in CI environments

Visual tests have been separated from functional tests to provide flexibility in testing workflows.

## File Structure

- `main.rs` - Server setup and routing
- `pages.rs` - Page handlers for each component
- `skeleton.rs` - Page template wrapper
- `livereload.rs` - WebSocket live reload functionality

## Component Development Pattern

1. Create/modify component in `../components/src/`
2. Add page route in `main.rs`
3. Create page handler in `pages.rs`
4. Use WebFetch to view rendered component during development
5. Test component variations and edge cases
