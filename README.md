This 10-day tutorial guides users from Python to high-performance web development with Rust and Actix Web:

- **[Day 1](Day1.md): Mental Model & Syntax** – AOT compilation, static typing, immutability, and pattern matching.
- **[Day 2](Day2.md): Ownership & Borrowing** – Memory safety via ownership laws, moving, and borrowing rules.
- **[Day 3](Day3.md): Data Modeling & Errors** – Structs, Enums (ADTs), and replacing exceptions with `Option` and `Result`.
- **[Day 4](Day4.md): Traits & Generics** – Interfaces and polymorphism, specifically the `FromRequest` trait for extractors.
- **[Day 5](Day5.md): Cargo & Architecture** – Project management with Cargo and modular code organization (`mod`/`pub`).
- **[Day 6](Day6.md): Async & Actix Basics** – Async/await, the Tokio runtime, and launching a multi-threaded HTTP server.
- **[Day 7](Day7.md): Routing & Extractors** – Type-safe request handling using `web::Path`, `web::Query`, `web::Json`, and Serde.
- **[Day 8](Day8.md): Database & State** – Compile-time SQL validation with SQLx and shared state via `web::Data`.
- **[Day 9](Day9.md): Middleware & Auth** – Request pipelines, `Logger` middleware, and header-based authentication guards.
- **[Day 10](Day10.md): Testing & Production** – In-memory integration tests, `--release` optimizations, and multi-stage Docker builds.
