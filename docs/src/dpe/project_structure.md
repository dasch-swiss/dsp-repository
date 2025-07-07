# Project Structure

> [!WARNING]  
> This page is not up-to-date.

## Workspace Layout

```
.
├── Cargo.toml         # Workspace manifest
├── types/             # Domain models, traits, and shared errors
├── services/          # Pure business logic implementations
├── storage/           # Data persistence (DB, in-memory) implementations
├── html_api/          # HTML routes, templates, and SSE endpoints
├── json_api/          # JSON RPC/REST endpoints
├── server/            # Web server binary crate
└── cli/               # CLI binary crate for tools and scripts
```

---

## Crate Responsibilities

### `types/` – Domain Types and Interfaces

This crate defines all core **data structures**, **error types**, and **trait interfaces** shared across the application. It is dependency-light and logic-free.

- Domain models (`ProjectCluster`, `ResearchProject`, `Collection`, `Dataset`, `Record`, etc.)
- Error types (`AppError`)
- Trait definitions:
    - Service traits: `MetadataService`
    - Repository traits: `MetadataRepository`

> Example:
```rust
pub trait MetadataRepository {
  async fn find_by_id(&self, id: &str) -> Result<ResearchProject, AppError>;
}
```

---

### `services/` – Business Logic

Implements the `types::service` traits and contains all **pure application logic**.

- Depends on `types`
- Free of side effects and I/O
- Easily testable
- Orchestrates workflows and enforces business rules

> Example:
```rust
pub struct MetadataServiceImpl<R: MetadataRepository> {
    pub repo: R,
}

#[async_trait]
impl<R: MetadataRepository> MetadataService for MetadataServiceImpl<R> {
    async fn find_by_id(&self, id: &str) -> Result<ResearchProject, AppError> {
        self.repo.find_by_id(id).await
    }
}
```

---

### `storage/` – Persistence Layer

Implements the `types::storage` traits to access data in external systems such as SQLite or in-memory stores.

- No business logic
- Easily swappable with mocks or test implementations

> Example:
```rust
pub struct InMemoryMetadataRepository { ... }

#[async_trait]
impl MetadataRepository for InMemoryMetadataRepository { 
  async fn find_by_id(&self, id: &str) -> Result<ResearchProject, AppError> {
    // In-memory lookup logic
  }
}
```

---

### `html_api/` – HTML Hypermedia + SSE

Handles the **user-facing UI** layer, serving HTML pages and fragments, and live-updating data via SSE.

- [Maud](https://maud.lambda.xyz/) templates
- [Datastar](https://docs.rs/datastar) for link generation
- SSE endpoints for live features like notifications, progress updates
- Routes for page rendering and form submissions

> Example:
```rust
#[get("/users/:id")]
async fn user_profile(State(app): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let user = app.user_service.get_user_profile(id).await?;
    HtmlTemplate(UserProfileTemplate { user })
}
```

---

### `http_api/` – Machine-Readable HTTP API

Exposes your application logic through a structured JSON API for integration with JavaScript frontends or third-party services.

- Cleanly separates business logic from representation
- Handles serialization and input validation

> Example:
```rust
#[get("/api/users/:id")]
async fn get_user(State(app): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let user = app.user_service.get_user_profile(id).await?;
    Json(user)
}
```

---

### `server/` – Web Server Binary

This crate is the **entrypoint** for running the full web application.

- Loads configuration
- Initializes services and storage
- Combines all route layers (`html_api`, `http_api`)
- Starts the Axum server

> Example:
```rust
#[tokio::main]
async fn main() -> Result<(), AppError> {
    let storage = PostgresUserRepository::new(...);
    let service = UserServiceImpl { repo: storage };

    let app = Router::new()
        .merge(html_api::routes(service.clone()))
        .merge(http_api::routes(service.clone()));

    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}
```

---

### `cli/` – Command-Line Interface

Provides a CLI for administrative or batch tasks such as:

- Import/export of data
- Cleanup scripts
- Background migrations
- Developer utilities

> Example (using [clap](https://docs.rs/clap)):
```rust
#[derive(Parser)]
enum Command {
    ImportUsers { file: PathBuf },
    ReindexSearch,
}
```

> Run via:
```
cargo run --bin cli -- import-users ./users.csv
```

---

## Benefits of This Structure

| Aspect                     | Benefit                                                                                  |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| **Separation of concerns** | Clear boundaries between domain, logic, persistence, and delivery                        |
| **Modular**                | Each crate can be tested and reused independently                                        |
| **Team-friendly**          | Frontend-focused devs work in `html_api`; backend devs focus on `services` and `storage` |
| **Testable**               | Services and repositories can be mocked for unit/integration testing                     |
| **Extensible**             | Add more APIs (e.g., GraphQL, CLI commands) without modifying existing code              |

---

## Development Guidelines

- **Never put business logic in route handlers.** Use the service layer.
- **Keep domain models and interfaces free of framework dependencies.**
- **Each crate has a single responsibility.**
- **SSE endpoints live in `html_api`, not as a separate API crate.**
- **Prefer async traits for I/O-related operations.**
- **Write integration tests in the same crate or create a top-level `tests/` crate for system-wide tests.**

---

## Future Growth Possibilities

- Add a `worker/` crate for background jobs
- Add a `scheduler/` crate for periodic tasks
- Add a `tests/` crate for orchestrated integration tests
- Add a `graphql_api/` or `admin_api/` if needed

---

## Getting Started

To run the application server:

```
cargo run --bin server
```

To run the CLI:

```
cargo run --bin cli -- help
```

---

## Summary

This modular design ensures clarity, maintainability, and smooth collaboration for both backend and frontend developers. The split between crates follows clean architecture principles and allows for focused development, rapid iteration, and clear testing strategies.
