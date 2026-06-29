# Day 9: Middleware, State, and Authentication

Welcome to Day 9! Today, we are focusing on protecting our endpoints and intercepting requests before they reach our core business logic.

In Python frameworks like FastAPI or Django, you write middleware by wrapping functions, using ASGI/WSGI layers, or creating decorators like `@login_required`. Python handles this dynamically at runtime by intercepting the request object and injecting properties or throwing exceptions.

In Actix Web, **Middleware** is built around a powerful, composable trait system called **Tower** (specifically, the service-oriented model of `Transform` and `Service`). Actix comes with robust built-in middleware for common tasks like logging, CORS, and sessions. Today, you will learn how to plug in standard logging middleware and use explicit application state checking to build a fast, type-safe JSON Web Token (JWT) style authentication guard.

---

## 🛡️ The Request Pipeline: How Middleware Fits In

When an HTTP request hits your Actix server, it doesn't drop straight into your handler. It passes through an onionskin pipeline of middleware layers first.

1. **Request Phase:** The middleware intercepts the request. It can inspect headers, modify data, or short-circuit the entire pipeline by returning an early error response (e.g., if authentication fails).
2. **Handler Phase:** If the middleware passes the request forward, your async handler executes its core database or processing logic.
3. **Response Phase:** On the way back out, the middleware can modify response headers, format errors, or inject logging metrics.

---

## 💻 Day 9 Practical Exercise: Building an Auth Guard with Middleware and State

Today, you will configure a server that uses Actix's built-in `Logger` middleware to trace incoming requests, and build a type-safe authentication guard that manually verifies an API key header.

### Step 1: Set up dependencies

Initialize a new project named `day9_middleware`:

```bash
cargo new day9_middleware
cd day9_middleware

```

Open your `Cargo.toml` and add `actix-web` and `env_logger` (a standard logging utility that formats system logs cleanly in your terminal console):

```toml
[dependencies]
actix-web = "4"
env_logger = "0.11"

```

### Step 2: Write the Protected Routing Code

Replace the contents of `src/main.rs` with the following code. Notice how we extract the headers directly using `web::Header` or inspect them manually inside a custom validation scope:

```rust
use actix_web::{get, middleware::Logger, web, App, HttpRequest, HttpResponse, HttpServer, Responder};

// Handler A: Public endpoint accessible by anyone
#[get("/public")]
async fn public_route() -> impl Responder {
    HttpResponse::Ok().body("🔓 This endpoint is completely public.")
}

// Handler B: Secret endpoint guarded by an API Key token check
#[get("/secret")]
async fn secret_route(req: HttpRequest) -> impl Responder {
    // 1. Inspect the incoming HTTP request headers manually
    match req.headers().get("X-API-Key") {
        Some(header_value) => {
            // Convert header bytes to a string slice safely
            if let Ok(token) = header_value.to_str() {
                if token == "super-secret-token-123" {
                    return HttpResponse::Ok().body("👑 Welcome to the secure admin control panel!");
                }
            }
            // Token was present but invalid
            HttpResponse::Unauthorized().body("❌ Invalid Authorization Token.")
        }
        None => {
            // Token was completely missing
            HttpResponse::BadRequest().body("❌ Missing required 'X-API-Key' header.")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 2. Initialize the environment logger terminal output (similar to Python's logging.basicConfig)
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    println!("🚀 Launching Middleware & Security server at http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            // 3. Register the built-in Logger middleware wrap.
            // This automatically logs every incoming request status code and execution time.
            .wrap(Logger::default())
            // 4. Register our API route services
            .service(public_route)
            .service(secret_route)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

```

### Step 3: Run and Test Security Layers

Run your server:

```bash
cargo run

```

Open a separate terminal window and test your access boundaries using `curl`. Watch your server terminal console update with real-time middleware log output for every request:

* **Test the public pipeline:**
```bash
curl -i http://127.0.0.1:8080/public

```


* **Test the secret pipeline with a missing header:**
```bash
curl -i http://127.0.0.1:8080/secret

```


*Observe the `400 Bad Request` block thrown before any internal processing happens.*
* **Test the secret pipeline with a faulty header token:**
```bash
curl -i -H "X-API-Key: wrong-token" http://127.0.0.1:8080/secret

```


* **Test the secret pipeline with the correct token payload:**
```bash
curl -i -H "X-API-Key: super-secret-token-123" http://127.0.0.1:8080/secret

```



---

## 🎯 Today's Mental Checklist

1. Do I see how Actix middleware handles data transformation *both* on the request incoming phase and response outgoing phase?
2. What are the core security benefits of evaluating authorization metadata before unpacking complex business logic models?
3. How does Actix's `.wrap()` syntax layer functionality incrementally over an existing application core factory?
