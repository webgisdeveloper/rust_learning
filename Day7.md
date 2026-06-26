# Day 7: Routing, Request Handling, and Extractors

Welcome to Day 7! Today, we are deep-diving into how Actix Web handles incoming data.

In Python frameworks like FastAPI or Flask, request parameters are often parsed dynamically or through validation libraries like Pydantic. If a client sends a malformed body or missing query string parameter, your code might start executing before crashing, or the framework handles the validation validation implicitly using Python's runtime type hints.

Actix Web leverages Rust's type system to handle this at the compiler level using a mechanism called **Extractors**. Extractors are types that know how to parse themselves out of an incoming HTTP request. If the data doesn't match your exact type definitions, Actix blocks the request and returns a clean `400 Bad Request` before your handler function code ever runs.

---

## 🧲 The Core Actix Extractors

Actix Web provides several built-in types to extract data from different parts of an HTTP request:

| Extractor | Source | Typical Python/FastAPI equivalent | Purpose |
| --- | --- | --- | --- |
| `web::Path<T>` | URL Path Segments | `path: int` / `hello/{name}` | Extracts dynamic parameters out of the endpoint path. |
| `web::Query<T>` | URL Query String | `q: Optional[str] = None` | Parses search or filter attributes from `?search=rust&page=2`. |
| `web::Json<T>` | HTTP Request Body | `payload: BaseModel` (Pydantic) | Deserializes JSON payloads into concrete Rust structs. |

---

## 🔄 Type-Safe Deserialization with Serde

To pull data into custom Rust structs using `web::Json` or `web::Query`, Actix relies on a powerful framework called **Serde** (Serialize/Deserialize).

By adding the attributes `#[derive(Deserialize)]` or `#[derive(Serialize)]` to a struct, Serde generates highly efficient parsing code at compile time. This completely bypasses the need for costly runtime reflection or dynamic object inspection.

---

## 💻 Day 7 Practical Exercise: Building a CRUD Request Router

Today, you are going to build a comprehensive set of API endpoints for managing products. This exercise will teach you how to extract path parameters, query parameters, and JSON payloads simultaneously.

### Step 1: Set up dependencies

Initialize a new project named `day7_extractors`:

```bash
cargo new day7_extractors
cd day7_extractors

```

Open your `Cargo.toml` and add both `actix-web` and `serde` (with its `derive` feature flag turned on):

```toml
[dependencies]
actix-web = "4"
serde = { version = "1.0", features = ["derive"] }

```

### Step 2: Write the Handler Code

Replace the contents of `src/main.rs` with the following code. Notice how clean the handler signatures look because the type definitions do all the hard extraction work:

```rust
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

// 1. Model the structure of an incoming JSON body for creating items
#[derive(Deserialize, Debug)]
struct CreateProduct {
    name: String,
    price: f64,
    inventory: u32,
}

// 2. Model the structure of an incoming URL Query parameter (?search=...&limit=...)
#[derive(Deserialize, Debug)]
struct ProductFilter {
    search: Option<String>,
    limit: Option<usize>,
}

// 3. Model a structure that we will send BACK to the client as a JSON response
#[derive(Serialize)]
struct ProductResponse {
    id: u64,
    name: String,
    price: f64,
}

// Handler A: POST /products (Extracts JSON payload)
#[post("/products")]
async fn create_product(payload: web::Json<CreateProduct>) -> impl Responder {
    println!("📥 Received request to create product: {:?}", payload);
    
    // Create a mock response object
    let response = ProductResponse {
        id: 991,
        name: payload.name.clone(),
        price: payload.price,
    };

    // HttpResponse::Created() returns a 201 status code
    // web::Json(response) automatically serializes our struct back to an HTTP JSON string
    HttpResponse::Created().json(response)
}

// Handler B: GET /products/{id} (Extracts dynamic URL Path)
#[get("/products/{id}")]
async fn get_product_by_id(path: web::Path<u64>) -> impl Responder {
    let product_id = path.into_inner(); // Unwraps the inner u64 value from the extractor container
    println!("🔍 Searching for product ID: {}", product_id);

    if product_id == 404 {
        return HttpResponse::NotFound().body("Product not found");
    }

    let response = ProductResponse {
        id: product_id,
        name: String::from("High-Performance Rust Textbook"),
        price: 49.99,
    };

    HttpResponse::Ok().json(response)
}

// Handler C: GET /products (Extracts URL Query params)
#[get("/products")]
async fn list_products(query: web::Query<ProductFilter>) -> impl Responder {
    println!("📋 Listing products with filters applied: {:?}", query);
    HttpResponse::Ok().body("Product filter queries executed successfully.")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting extractor server on http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .service(create_product)
            .service(get_product_by_id)
            .service(list_products)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

```

### Step 3: Test and Break Your Endpoints

Run the server with `cargo run`. Open a separate terminal shell to test how strictly the Extractors protect your handlers:

* **Test valid path extraction:**
```bash
curl http://127.0.0.1:8080/products/125

```


* **Test invalid path extraction (Pass letters instead of a number):**
```bash
curl http://127.0.0.1:8080/products/abc

```


*Observe that Actix intercepts this and automatically sends back a `400 Bad Request` without hitting your internal println logs.*
* **Test valid JSON creation payload:**
```bash
curl -X POST http://127.0.0.1:8080/products \
     -H "Content-Type: application/json" \
     -d '{"name": "Mechanical Keyboard", "price": 120.50, "inventory": 14}'

```



---

## 🎯 Today's Mental Checklist

1. Do I see how `#[derive(Deserialize)]` hooks into the Actix extraction layer to parse raw data before handler invocation?
2. What happens to an incoming request if a user misses a non-optional field inside a `web::Json<T>` data model?
3. Can I explain the syntactic step of using `.into_inner()` on an Actix web extractor?
