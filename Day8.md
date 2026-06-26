# Day 8: Database Integration (SQLx) & Application State

Welcome to Day 8! Today, we are bridging your async Actix Web handlers to a persistent data layer using **SQLx**, and learning how to pass shared resources safely across multiple OS threads.

In Python frameworks like Django or FastAPI (with SQLAlchemy), you typically use an Object-Relational Mapper (ORM) that executes queries dynamically at runtime. If you have a typo in a column name or a type mismatch, you won't find out until that specific line of Python code executes at runtime.

**SQLx** is an async, pure-Rust SQL crate that does something incredible: it connects to your live database **at compile time** to validate your SQL syntax and guarantee that your database schema exactly matches your Rust structs. If a column name is misspelled, your app won't even compile.

---

## 🧵 The Multithreaded State Challenge

As you learned on Day 6, Actix Web spins up an independent HTTP worker thread on every CPU core. This creates a architectural puzzle: If Thread 1 and Thread 4 both need to use the same database connection pool, how do we share that memory safely without causing race conditions?

In Python, the Global Interpreter Lock (GIL) prevents threads from running truly concurrently, masking these safety issues. In Rust, the compiler forces us to be explicit. Actix Web solves this through its dependency injection container: **`web::Data`**.

When you wrap a database pool inside `web::Data`, Actix utilizes atomic reference counting (`Arc`) under the hood to safely clone read-only access to that resource across every single thread worker core.

---

## 💻 Day 8 Practical Exercise: Wiring an Async DB Pool into Actix

Today, you will configure a mock database setup that mimics exactly how a production SQLite or PostgreSQL connection pool is injected into an Actix Web application state context.

### Step 1: Initialize your project & add dependencies

Create a new project named `day8_database`:

```bash
cargo new day8_database
cd day8_database

```

Open `Cargo.toml` and add `actix-web`, `serde` (for handling JSON), and `tokio` (so we can use its async mutex locking features for this mock setup):

```toml
[dependencies]
actix-web = "4"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["sync"] }

```

### Step 2: Write the Code

Replace the contents of `src/main.rs`. We will build a mock in-memory "database database pool" using an async Mutex wrapper so you can see how state maps across web requests without needing an external database server running today:

```rust
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// 1. Model our database row state
#[derive(Clone, Serialize, Deserialize)]
struct Book {
    id: usize,
    title: String,
    author: String,
}

// 2. Define our shared Application State container
// In a production app, this would wrap an `sqlx::PgPool` or `sqlx::SqlitePool` directly.
struct AppState {
    app_name: String,
    mock_db: Mutex<Vec<Book>>, // Mutex allows safe, mutable access across multiple threads
}

#[derive(Deserialize)]
struct NewBookRequest {
    title: String,
    author: String,
}

// Handler A: GET /books - Extracts our shared state using web::Data
#[get("/books")]
async fn list_books(data: web::Data<AppState>) -> impl Responder {
    // Lock the mutex safely across the async await point to read the database rows
    let books = data.mock_db.lock().await;
    
    println!("📡 [{}] Fetching all books from mock DB pool...", data.app_name);
    HttpResponse::Ok().json(&*books)
}

// Handler B: POST /books - Extracts BOTH state and a JSON payload
#[post("/books")]
async fn create_book(
    data: web::Data<AppState>,
    payload: web::Json<NewBookRequest>,
) -> impl Responder {
    let mut books = data.mock_db.lock().await;
    let new_id = books.len() + 1;

    let new_book = Book {
        id: new_id,
        title: payload.title.clone(),
        author: payload.author.clone(),
    };

    books.push(new_book.clone());
    println!("💾 [{}] Inserted book ID: {} successfully.", data.app_name, new_id);

    HttpResponse::Created().json(new_book)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 3. Initialize our state BEFORE starting the HTTP server
    // We wrap it inside web::Data here to create the shared pointer container
    let shared_state = web::Data::new(AppState {
        app_name: String::from("Actix-SQLx-Demo"),
        mock_db: Mutex::new(vec![
            Book { id: 1, title: String::from("The Rust Programming Language"), author: String::from("Steve Klabnik") }
        ]),
    });

    println!("🚀 Server running at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            // 4. Inject the state pointer into our application factory instance.
            // This makes it accessible as a web::Data extractor argument to all handlers.
            .app_data(shared_state.clone()) 
            .service(list_books)
            .service(create_book)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

```

### Step 3: Run and Verify Thread Safety

Run the server:

```bash
cargo run

```

Open a second terminal shell and query your endpoints to watch state modify safely over concurrent async tasks:

* **Fetch initial database records:**
```bash
curl http://127.0.0.1:8080/books

```


* **Insert a new database record:**
```bash
curl -X POST http://127.0.0.1:8080/books \
     -H "Content-Type: application/json" \
     -d '{"title": "Programming Rust", "author": "Jim Blandy"}'

```


* **Verify insertion persisted in memory across thread workers:**
```bash
curl http://127.0.0.1:8080/books

```



---

## 🎯 Today's Mental Checklist

1. Why can't we use standard global mutable variables for a database reference pool in Rust like we often do in Python?
2. What role does `web::Data` fill inside the Actix architecture, and how does it correlate with dependency injection?
3. What is the fundamental safety benefit of using a compile-time checked database layer like SQLx over a standard dynamic ORM?
