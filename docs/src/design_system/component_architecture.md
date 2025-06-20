# Component Architecture Decision

## Composability Approach

We use a **Maud-Native + Props** combination for component composability:

### Core Principles

1. **Maud `Markup` Return Types**: Components return `maud::Markup` instead of `String` for zero-copy composition
2. **Props Structs**: Complex components use dedicated props structs for clear parameter grouping
3. **Simple Functions**: Basic components remain as simple functions
4. **Flexible Text Input**: Use `impl Into<String>` for text parameters

### Component Patterns

#### Simple Components
```rust
use maud::{Markup, html};

pub fn button(text: impl Into<String>) -> Markup {
    html! {
        button .dsp-button { (text.into()) }
    }
}
```

#### Complex Components with Props
```rust
pub struct CardProps {
    pub title: String,
    pub content: Markup,
    pub variant: CardVariant,
}

pub fn card(props: CardProps) -> Markup {
    html! {
        div .dsp-card {
            h2 .dsp-card__title { (props.title) }
            div .dsp-card__content { (props.content) }
        }
    }
}
```

#### Component Composition
```rust
// Direct nesting - zero-copy composition
pub fn page_header(title: impl Into<String>, actions: Markup) -> Markup {
    html! {
        header .dsp-page-header {
            h1 { (title.into()) }
            div .dsp-page-header__actions {
                (actions)  // Direct Markup insertion
            }
        }
    }
}
```

### Benefits

- **Efficient**: No string concatenation overhead
- **Type Safe**: Compile-time guarantees for component structure  
- **Composable**: Components nest naturally without conversion
- **Extensible**: Props structs make adding parameters easy
- **Consistent**: Unified approach across all components

### Migration Path

1. Convert existing components to return `Markup`
2. Add props structs for components with 3+ parameters
3. Use `impl Into<String>` for text inputs
4. Test composition in playground