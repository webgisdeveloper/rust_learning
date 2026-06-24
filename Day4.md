# Day 4: Traits, Generics, and the Actix Extractor Architecture

Welcome to Day 4! Today we are tackling how Rust creates abstractions.

In Python, you rely heavily on **duck typing** ("if it walks like a duck and quacks like a duck, it's a duck") and inheritance. If an object has a `.to_json()` method, you can pass it to a function that expects a JSON string, and everything works—until someone passes a type that lacks that method, triggering a runtime `AttributeError`.

Rust replaces duck typing and subclassing with **Traits** and **Generics**. This combination allows you to write highly reusable, polymorphic code while ensuring absolute type safety at compile time. It is also the exact engine that powers **Extractors**, the most critical syntactic sugar in Actix Web.

---

## 🧬 Understanding Traits (Rust’s Interfaces)

A **Trait** defines a specific interface or capability that a type must promise to implement.

```rust
// Defining a trait (a contract)
trait Serializable {
    fn to_json(&self) -> String;
}

struct User { username: String }

// Implementing that trait for our specific struct
impl Serializable for User {
    fn to_json(&self) -> String {
        format!("{{\"username\": \"{}\"}}", self.username)
    }
}

```

If a type implements `Serializable`, you can pass it to any function that requires that capability. If it doesn't, the compiler rejects the code before it runs.

---

## 📦 Generics & Trait Bounds

Generics allow you to write functions or structs that work over multiple concrete data types without duplicating code. In Rust, you use **trait bounds** to tell the compiler: *"This function accepts any type `T`, as long as `T` implements this specific trait."*

```rust
// This function accepts ANY type T, provided T implements Serializable
fn print_payload<T: Serializable>(payload: T) {
    println!("Payload JSON: {}", payload.to_json());
}

```

---

## 🔍 The Actix Web Secret: The `FromRequest` Trait

Why are we learning this today? Because Actix Web’s entire routing layer is built on a single, powerful trait: `FromRequest`.

When you write an Actix handler like this:

```rust
// Preview of Actix Web syntax
async fn create_user(item: web::Json<User>) -> HttpResponse { ... }

```

`web::Json<T>` is a generic struct. It implements Actix's `FromRequest` trait. When an HTTP request hits your server, Actix looks at your function signatures, sees `web::Json<User>`, and calls the trait method behind the scenes to automatically parse, validate, and convert the raw HTTP string payload into your concrete `User` struct.

---

## 💻 Day 4 Practical Exercise: Building a Mock Actix Extractor

Today, you will build a mini architecture that mimics how Actix Web uses traits and generics to automatically parse types out of "incoming web requests."

### Step 1: Initialize your project

```bash
cargo new day4_traits
cd day4_traits

```

### Step 2: Replace `src/main.rs`

Copy this code into your project. Pay close attention to how the generic function `extract_and_process` uses trait bounds to handle different types of data.

```rust
// 1. Define our mock "HTTP Request" containing a string payload
struct HttpRequest {
    payload: String,
}

// 2. Define the Extractor trait (similar to Actix Web's FromRequest)
trait FromRequest {
    // A trait method that takes a request and tries to extract Self
    fn extract(req: &HttpRequest) -> Result<Self, String> where Self: Sized;
}

// 3. Define two different structs we might want to extract from a web request
struct UserIdExtractor(u64);
struct SearchQueryExtractor(String);

// 4. Implement the trait for the User ID extractor (expects a plain number string)
impl FromRequest for UserIdExtractor {
    fn extract(req: &HttpRequest) -> Result<Self, String> {
        match req.payload.trim().parse::<u64>() {
            Ok(id) => Ok(UserIdExtractor(id)),
            Err(_) => Err(String::from("400 Bad Request: Invalid User ID numeric format")),
        }
    }
}

// 5. Implement the trait for the Search Query extractor
impl FromRequest for SearchQueryExtractor {
    fn extract(req: &HttpRequest) -> Result<Self, String> {
        if req.payload.is_empty() {
            Err(String::from("400 Bad Request: Search query cannot be empty"))
        } else {
            Ok(SearchQueryExtractor(req.payload.clone()))
        }
    }
}

// 6. A Generic "Route Handler" that handles ANY extractor type T
fn extract_and_process<T: FromRequest>(req: HttpRequest, handler: fn(T)) {
    match T::extract(&req) {
        Ok(extracted_data) => handler(extracted_data),
        Err(err_msg) => println!("❌ Extractor Failed: {}", err_msg),
    }
}

fn main() {
    // Scenario A: A request hitting a GET /user/{id} endpoint
    let req_a = HttpRequest { payload: String::from("10523") };
    println!("--- Routing Request A (User ID) ---");
    extract_and_process::<UserIdExtractor>(req_a, |data| {
        println!("✓ Success! Routing to dashboard for User ID: {}", data.0);
    });

    // Scenario B: A request hitting a search endpoint with invalid data
    let req_b = HttpRequest { payload: String::from("") };
    println!("\n--- Routing Request B (Search Query) ---");
    extract_and_process::<SearchQueryExtractor>(req_b, |data| {
        println!("✓ Success! Performing database search for: {}", data.0);
    });
}

```

### Step 3: Run the Application

```bash
cargo run

```

Observe how `extract_and_process` acts exactly like an Actix Web engine framework route. It doesn't care what data type it is processing, as long as that data type knows how to parse itself via the `FromRequest` contract.

---

## 🎯 Today's Mental Checklist

1. Can I explain why Rust’s traits are safer than Python’s structural duck typing?
2. When looking at a generic signature like `fn handle<T: FromRequest>(val: T)`, what does `<T: FromRequest>` explicitly mean?
3. Do I understand how Actix Web uses this system to automatically transform raw HTTP strings into ready-to-use Rust structs?
