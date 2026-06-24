# Day 3: Structs, Enums, and Robust Error Handling

Welcome to Day 3! Today, we are bridging the gap between Rust’s type system and your day-to-day backend patterns.

In Python web frameworks, you rely heavily on `try/except` blocks to catch runtime errors (like database connection drops or validation failures), and you use libraries like Pydantic or native dataclasses to model data payloads.

Rust ditches exceptions entirely. Instead, it treats **errors as values** using powerful types called `Result` and `Option`. Together with `Structs` and `Enums`, these form the backbone of how Actix Web handles request payloads and returns HTTP status codes cleanly.

---

## 🏗️ Data Modeling: Structs vs. Enums

### 1. Structs (The Data Builders)

Structs in Rust are akin to typed Python Dataclasses or Pydantic models. They group related fields together.

```rust
struct User {
    id: u64,
    username: String,
    is_active: bool,
}

```

### 2. Enums (The Type Superpower)

In Python, an Enum is just a set of named constants. In Rust, Enums are **Algebraic Data Types (ADTs)**, meaning each enum variant can hold its own distinct payload of data. This is incredibly useful for capturing state in web APIs.

```rust
enum UserRole {
    Admin,                               // Simple variant
    Manager { region: String },          // Variant with named fields
    Guest(u32),                          // Variant with a tuple payload (e.g., hours remaining)
}

```

---

## ⚠️ The Death of `NoneType` and Exceptions

If you've spent time in Python, you've definitely run into this runtime nightmare:
`AttributeError: 'NoneType' object has no attribute 'get'`

Rust solves this at compile time by eliminating the concept of a null/None pointer entirely. Instead, it uses two standard library enums: `Option` and `Result`.

### 1. The `Option` Enum: Handling Absence

When a value might be missing (e.g., a query parameter that wasn’t passed, or a user not found in the database), Rust forces you to wrap it in an `Option`.

```rust
enum Option<T> {
    Some(T), // Contains the value
    None,    // Contains absolutely nothing
}

```

To get the value out, you *must* unpack it using pattern matching or built-in helper methods. The compiler will not let you accidentally treat a `None` value as a valid object.

### 2. The `Result` Enum: Robust Error Handling

Instead of throwing a runtime exception that bubbles up and potentially crashes your worker thread, functions that can fail return a `Result`.

```rust
enum Result<T, E> {
    Ok(T),  // The operation succeeded; holds the successful data
    Err(E), // The operation failed; holds the error details
}

```

---

## 💻 Day 3 Practical Exercise: The Safe API Payload Parser

When you build endpoints in Actix Web, incoming data can be corrupt, missing, or unauthorized. Today, you will build a mock request handler that takes a raw payload, validates it using `Result` and `Option`, and maps it safely without a single `try/except` block.

### Step 1: Initialize your project

```bash
cargo new day3_error_handling
cd day3_error_handling

```

### Step 2: Replace `src/main.rs`

Copy the following code into your project. Read the `match` blocks carefully to see how we unpack values cleanly.

```rust
// A mock representation of a web request payload
struct ApiRequest {
    body: Option<String>,
    auth_token: Option<String>,
}

#[derive(Debug)]
enum ApiError {
    MissingBody,
    InvalidToken,
}

// A mock struct for our authenticated application user
#[derive(Debug)]
struct AuthenticatedUser {
    username: String,
}

fn main() {
    // Scenario A: A valid incoming request
    let good_request = ApiRequest {
        body: Some(String::from("{'item': 'laptop'}")),
        auth_token: Some(String::from("valid_secret_jwt")),
    };

    // Scenario B: An unauthorized request missing an auth token
    let bad_request = ApiRequest {
        body: Some(String::from("{'item': 'phone'}")),
        auth_token: None,
    };

    println!("--- Processing Scenario A ---");
    match handle_request(good_request) {
        Ok(user) => println!("✓ Request Success! Logged in as: {:?}", user),
        Err(err) => println!("❌ Request Failed with error: {:?}", err),
    }

    println!("\n--- Processing Scenario B ---");
    match handle_request(bad_request) {
        Ok(user) => println!("✓ Request Success! Logged in as: {:?}", user),
        Err(err) => println!("❌ Request Failed with error: {:?}", err),
    }
}

// A function that attempts to authenticate a request, returning a Result
fn handle_request(req: ApiRequest) -> Result<AuthenticatedUser, ApiError> {
    // 1. Unwrapping the auth_token Option
    let token = match req.auth_token {
        Some(t) => t,
        None => return Err(ApiError::InvalidToken), // Early return with an Err value!
    };

    // 2. Unwrapping the body Option
    if req.body.is_none() {
        return Err(ApiError::MissingBody);
    }

    // 3. Mock verification logic
    if token == "valid_secret_jwt" {
        Ok(AuthenticatedUser {
            username: String::from("Dev_Jun_Wang"),
        })
    } else {
        Err(ApiError::InvalidToken)
    }
}

```

### Step 3: Run and Observe

Run the code via your terminal:

```bash
cargo run

```

Notice how control flow works explicitly through returned data. In Actix Web, you will use this exact same pattern. Actix provides a built-in trait called `ResponseError` that allows you to easily map your custom `ApiError` enum variants directly to actual HTTP status codes (like `400 Bad Request` or `401 Unauthorized`).

---

## 🎯 Today's Mental Checklist

1. Do I see how Rust's `Option` forces me to think about missing data *before* my code executes?
2. If an Actix endpoint hits an unreachable database, why is returning an `Err(DatabaseError)` safer than letting a Python exception bubble up?
3. Can I explain the difference between a Struct holding fixed data fields and an Enum holding variant states?
