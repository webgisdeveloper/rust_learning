# Day 5: Cargo, Crate Architecture, and Project Structure

Welcome to Day 5! You’ve made it to the end of your first week. Today, we shift our focus from core language syntax to project management, code organization, and compilation tools.

In the Python ecosystem, project configuration can feel fractured. You might use `pip` for packages, `venv` or `virtualenv` for isolation, `poetry` or `pipenv` for dependency management, and `setuptools` or `flit` for packaging.

Rust consolidates all of this into a single, world-class tool: **Cargo**. Cargo is your package manager, build system, test runner, and documentation generator all rolled into one. Today, we will learn how to structure a modular Rust project using Cargo, preparing a clean layout for the Actix Web application you will build next week.

---

## 📦 The Anatomy of Cargo

When you run `cargo new my_project`, Cargo generates a standard directory layout:

```text
my_project/
├── Cargo.toml      # Project manifest (metadata, dependencies, build profiles)
├── Cargo.lock      # Pinpointed exact versions of upstream dependencies
└── src/
    └── main.rs     # The root source file for a binary application

```

### `Cargo.toml` vs. `Cargo.lock`

* **`Cargo.toml`:** This is written by *you*. It contains semantic version requirements for your dependencies (similar to `pyproject.toml` or `requirements.txt`).
* **`Cargo.lock`:** This is managed automatically by *Cargo*. It tracks the exact, cryptographic hash and version of every package down the dependency tree. This ensures that if your project builds on your machine today, it will build exactly the same way on a production server or a colleague's machine.

---

## 🗂️ The Rust Module System (`mod` and `pub`)

In Python, file layout explicitly dictates your module layout (e.g., importing a file named `auth.py` is automatically done via `import auth`).

Rust is different. **File paths do not automatically map to modules.** You must explicitly declare your module hierarchy inside your code using the `mod` keyword, and explicitly mark items as public using `pub` if they need to be accessed outside their own file. By default, everything in Rust is strictly private.

### The Standard Module Layout

To keep code clean as your Actix Web project grows, you will separate your application into distinct layers (e.g., routes, database models, configuration).

* **`main.rs`**: The entry point. It registers top-level modules.
* **`models.rs`**: A file module housing data structures.
* **`routes/`**: A directory module containing multiple routing files, managed by a `mod.rs` file.

---

## 💻 Day 5 Practical Exercise: Structuring a Modular Actix Skeleton

Today, you are going to build a multi-file, modular project skeleton. It won’t run an HTTP server yet, but it will implement the exact architecture pattern required for an Actix Web application.

### Step 1: Initialize a new binary project

```bash
cargo new day5_architecture
cd day5_architecture

```

### Step 2: Create the file structure

Inside your new project, create the following files and directories so your tree matches this layout:

```text
day5_architecture/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── models.rs
    └── routes/
        ├── mod.rs
        └── user_routes.rs

```

### Step 3: Write the Code

#### `src/models.rs`

Define a public data struct that your routing layer will eventually use.

```rust
// We must mark the struct AND its fields as `pub` to make them accessible outside this file
#[derive(Debug)]
pub struct UserProfile {
    pub username: String,
    pub role: String,
}

```

#### `src/routes/user_routes.rs`

Write a mock handler function that uses our `UserProfile` model.

```rust
// Use `super::super` to navigate back up to the root to access the models module
use crate::models::UserProfile;

pub fn mock_create_user_handler() {
    let new_user = UserProfile {
        username: String::from("Rustacean_Jun"),
        role: String::from("Admin"),
    };
    println!("📡 [Actix Mock Route] Successfully processed payload for: {:?}", new_user);
}

```

#### `src/routes/mod.rs`

This file acts as the gatekeeper for the `routes` directory. It declares `user_routes` as a submodule and exposes its functions.

```rust
// Declare the user_routes file as a submodule
pub mod user_routes;

```

#### `src/main.rs`

Tie everything together by declaring your modules at the root crate level and invoking the mock handler.

```rust
// 1. Explicitly declare top-level modules in main.rs
mod models;
mod routes;

fn main() {
    println!("--- Initializing Modular Project Architecture ---");

    // 2. Call the deeply nested module function using explicit paths
    routes::user_routes::mock_create_user_handler();
}

```

### Step 4: Run the Application

```bash
cargo run

```

---

## 🎯 Week 1 Recap & Verification

You have now completed Week 1! You have successfully configured a modular project compilation layout and understand how to manage visibility boundaries.

Before we move on to building real async Actix endpoints on Day 6, make sure you feel confident with these foundational shifts:

1. Do I understand why `pub mod` is required to expose code in subdirectories?
2. Can I explain why Cargo doesn't need an external virtual environment tool to manage local project dependencies?
3. Am I comfortable tracking down code visibility errors when the compiler says an item is private?
