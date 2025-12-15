# Example Pages

This crate contains example applications that showcase the design system components in realistic contexts. These examples serve multiple purposes: demonstrating component usage, identifying gaps in the component library, and showcasing DataStar integration patterns.

## Basic Website Example

A multi-page website demonstrating the design system in a real-world context, modeled after the DaSCH Swiss platform. The example showcases server-rendered HTML with hypermedia-driven interactivity via DataStar.

### Running the Example

```bash
# Run with hot reload (recommended for development)
just run-watch-example-basic-website

# Or run once
just run-example-basic-website
```

Visit http://localhost:3500 to view the application.

### Pages

- **Home** (`/`) - Landing page with hero, services, statistics, and news
- **Projects** (`/projects`) - Project listing with pagination and detail drawer
- **Services** (`/services`) - Service offerings and pricing
- **Knowledge Hub** (`/knowledge-hub`) - Article categories and resources
- **About Us** (`/about-us`) - Organization information and team
- **FAQ** (`/faq`) - Frequently asked questions with expandable sections
- **Contact** (`/contact`) - Contact information and office location
- **Status** (`/status`) - Live system status dashboard
- **News** (`/news`) - News listing (placeholder)

## DataStar Features Demonstrated

This example showcases various DataStar features for building hypermedia-driven applications:

### 1. Server-Sent Events (SSE)
- **Statistics Stream** (`/api/stats/stream`) - Live updating counters on home page
- **Status Stream** (`/api/status/stream`) - Real-time system health monitoring

### 2. Fragment Merging
- Targeted updates using element IDs without full page reloads
- Multiple simultaneous element updates from single SSE event
- Preserves page state during updates

### 3. Signals (Reactive State)
- `$drawerOpen` - Controls drawer visibility on projects page
- Client-side state management without JavaScript

### 4. Event Handlers
- `data-on-load` - Triggers actions when element loads (SSE connections)
- `data-on:click` - Handles click events for interactions

### 5. Conditional Classes
- `data-class:opacity-*` - Controls element opacity based on signal state
- `data-class:pointer-events-*` - Manages interactivity
- `data-class:translate-*` - Slide-in animations for drawer

### 6. Dynamic Content Loading
- `@get()` - Fetches HTML fragments from server
- Drawer content loaded on demand when project is clicked
- Loading states with graceful fallbacks

## Architecture

The example follows clean separation of concerns:

- **`api.rs`** - HTTP endpoints and SSE handlers
- **`layout.rs`** - Page layout and shell configuration
- **`pages/`** - Individual page implementations
- **`components/`** - Reusable UI components (local to this example)

Components are intentionally kept local to the example to allow experimentation. Production-ready components should be promoted to the main design system crate.

## Design System Integration

The example integrates with the main design system components:

- **Header** - Site navigation with responsive behavior
- **Footer** - Site footer with links
- **Shell** - Page wrapper integrating header and footer
- **Hero** - Hero section with headline and CTAs
- **Logo Cloud** - Partner logo display

Local components may be candidates for promotion to the design system after refinement.
