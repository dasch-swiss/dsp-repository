# DSP Design System

## Introduction

The DSP Design System is built with Tailwind as the foundation, providing a modern, utility-first approach to component design and styling.

The system is designed with the following principles:

- It is not a general purpose design system, but a purpose-built system. As such, it is much smaller and less complex.
- It is not generic or customizable, instead the DSP brand is baked into it, thus simplifying complexity of use.
- It is purposefully kept small:
  - It comes with two themes (dark and light) with no option for custom theming.
  - The set of available styles (colors, typography, spacing, etc.)
    is kept intentionally small to promote consistent user interfaces.
  - It only has the components that are strictly needed.
    Additional components may be added, when necessary.
  - It only has the component variants that are strictly needed.
    Additional component variants may be added, when necessary.
- It may have purpose-specific components tailored to the DSP platform's unique requirements.


# Current Implementation Status

The DSP Design System is currently in early development.

## Development Environment

### Playground
- **URL**: http://localhost:3400 (via `just run-watch-playground`)
- **Architecture**: Full-featured development environment with Rust server and TypeScript testing
- **Features**: 
  - **Shell Interface**: Sidebar navigation with component list and active states
  - **Component Isolation**: Iframe-based component rendering with parameter controls
  - **Variant Selection**: Dynamic component variant switching with real-time updates
  - **Theme Switching**: Light/dark theme toggle with live preview
  - **Documentation Tabs**: Component and documentation view switching
  - **Live Reload**: WebSocket-based automatic refresh on file changes
- **Testing**: Comprehensive TypeScript/Playwright test suite with functional, accessibility, and responsive tests
- **Commands**:
  - `just playground install` - Install frontend dependencies and browsers
  - `just playground test` - Run all automated tests
  - `just playground test-headed` - Run tests with browser visible

### Example Pages

The design system includes example applications that showcase components in realistic contexts. These examples demonstrate component usage, identify gaps in the component library, and showcase DataStar integration patterns.

#### Basic Website Example
- **URL**: http://localhost:3500 (via `just run-watch-example-basic-website`)
- **Purpose**: Multi-page website demonstrating the design system with server-rendered HTML and hypermedia-driven interactivity
- **Features**:
  - Complete website with 9 pages (home, projects, services, knowledge hub, about, FAQ, contact, status, news)
  - Server-Sent Events (SSE) for live updates (statistics and system status)
  - Interactive drawer with dynamic content loading
  - Pagination and search
  - Responsive design with mobile and desktop layouts
- **DataStar Integration**: Showcases SSE, fragment merging, signals, event handlers, conditional classes, and dynamic content loading
- **Commands**:
  - `just run-example-basic-website` - Run the example
  - `just run-watch-example-basic-website` - Run with hot reload

See `modules/design_system/example_pages/README.md` for detailed information about the example and the DataStar features demonstrated.

<!-- TODO: Add the following pages:
  - [Aim and Purpose]()
  - [Design Principles]()
  - [Design Tokens]()
  - [Components]()
  - [Patterns]()
  - [Icons]()
  - [Typography]()
  - [Colors]()
  - [Accessibility]()
  - [Design System in Figma]()
  - [Playground]()
 -->
