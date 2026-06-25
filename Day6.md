# Day 6: Async Rust, the Tokio Runtime, and Actix Web Basics

Welcome to Week 2! You have mastered the core syntax and safety rules of Rust. Now, it's time to build high-performance web systems.

In Python, frameworks like FastAPI, Sanic, or Tornado rely on an asynchronous event loop (via `asyncio`). Python's async model is single-threaded by default due to the Global Interpreter Lock (GIL). To utilize multiple CPU cores, you have to spin up multiple worker processes (e.g., via Gunicorn).

Rust does not have a GIL. Async Rust is **multi-threaded by default**. Today, we will explore how async works in Rust, how the `actix-rt` runtime engine handles tasks across CPU cores, and how to spin up your very first genuine Actix Web HTTP server.

---

## 🧠 The Async Shift: Python vs. Rust

While both languages use `async` and `await` keywords, their underlying mechanics are fundamentally different:

| Feature | Python (`asyncio`) | Rust (`Tokio` / `Actix`) |
| --- | --- | --- |
| **Execution Model** | Driven by a runtime event loop baked into the language or standard library. | The language provides *only* the syntax (`Future` trait). You must bring an external execution runtime (like Tokio). |
| **Threading** | Single-threaded event loop. CPU-bound work blocks the loop unless offloaded to a thread pool. | Multi-threaded work-stealing execution loop. Spreads network I/O tasks across all available CPU cores automatically. |
| **Futures** | Active upon creation. Calling an async function schedules it to run immediately. | **Lazy**. A Rust `Future` does absolutely nothing until you explicitly `.await` it or spawn it onto a runtime. |

---

## ⚙️ How Actix Web Generates Speed

Actix Web sits on top of a highly optimized runtime engine (`actix-rt`), which is built directly on top of **Tokio** (the industry-standard async engine for Rust).

When you initialize an Actix Web server, it inspects your machine's hardware and automatically spins up a separate single-threaded event loop worker for **every CPU core** available. This means your application can process tens of thousands of concurrent HTTP requests natively without ever fighting process-forking overhead or GIL resource locking.

---

## 💻 Day 6 Practical Exercise: Your First Actix HTTP Server

Today, you will transition from writing terminal CLI tools to launching a real asynchronous HTTP web server that responds to requests.

### Step 1: Update your dependencies

Initialize a new project called `day6_actix_basics`:

```bash
cargo new day6_actix_basics
cd day6_actix_basics

```

Open your `Cargo.toml` file and add `actix-web` under the dependencies block:

```toml
[dependencies]
actix-web = "4"

```

### Step 2: Write the Server Code

Replace the entire contents of `src/main.rs` with the code below. Read carefully to understand the routing macros and the wrapper function initialization:

```rust
use actix_web::{get, App, HttpResponse, HttpServer, Responder};

// 1. Define an asynchronous route handler using the `#[get]` macro
#[get("/")]
async fn hello_world() -> impl Responder {
    // HttpResponse::Ok() generates an HTTP 200 Status code
    HttpResponse::Ok().body("Hello from your multi-threaded Actix Web server!")
}

// 2. Define a second async route handler
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("{\"status\": \"healthy\"}")
}

// 3. The magic macro that sets up the async runtime engine under our main function
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Launching Actix Web server on http://127.0.0.1:8080");

    // 4. Instantiated the multi-threaded HTTP Server
    HttpServer::new(|| {
        // This closure runs for EVERY thread/CPU worker core Actix creates
        App::new()
            .service(hello_world)  // Register our "/" route
            .service(health_check) // Register our "/health" route
    })
    .bind(("127.0.0.1", 8080))?   // Bind to local host on port 8080
    .run()                        // Run the execution engine loop
    .await                        // Wait asynchronously for shutdown signals
}

```

### Step 3: Compile and Test

Run your application using Cargo:

```bash
cargo run

```

*(Note: The first compilation will take a few moments as Cargo downloads and builds the complete Actix and Tokio runtime trees).*

Once it says it's running, open your web browser or use a terminal curl command to hit your endpoints:

```bash
curl http://127.0.0.1:8080/
curl http://127.0.0.1:8080/health

```

---

## 🎯 Today's Mental Checklist

1. Why does a Rust `Future` need an explicit runtime engine macro (`#[actix_web::main]`) to actually execute, unlike a Python coroutine?
2. What does it mean that Actix runs an independent application context instance per CPU core inside its thread pool?
3. Am I comfortable using `async/await` syntax to write non-blocking code blocks?
