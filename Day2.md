# Day 2: Ownership, Borrowing, and Lifetimes

Welcome to Day 2! Today we are tackling **Ownership**—the single most unique concept in Rust.

In Python, you never have to think about when an object is deleted from memory. Python’s Garbage Collector (GC) runs in the background, keeping track of references and cleaning up when they drop to zero.

Rust has **no garbage collector**. Instead, memory is managed through a strict system of ownership with rules that the compiler enforces. This is how Rust achieves blazing-fast performance without manual memory management (`malloc`/`free`) and why it never suffers from Python's common runtime issues like threading race conditions or mutated shared state bugs.

---

## 🧠 The 3 Laws of Ownership

Every piece of data you create in Rust is governed by three strict rules:

1. **Each value in Rust has an owner** (usually a variable name).
2. **There can only be one owner at a time.**
3. **When the owner goes out of scope, the value is dropped** (memory is instantly freed).

### 1. The "Move" Behavior (vs. Python Copying)

In Python, if you assign a list to another variable, both labels point to the *same object* on the heap.

```python
# Python
list_a = [1, 2, 3]
list_b = list_a     # Both look at the same memory slot
list_b.append(4)
print(list_a)       # Output: [1, 2, 3, 4] -> list_a mutated unexpectedly!

```

In Rust, complex types that live on the heap (like `String` or `Vec`) don't share ownership. Setting one variable to another **moves** the data. The original variable becomes invalid immediately.

```rust
// Rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1; // The ownership of the string data has "Moved" to s2

    // println!("{}", s1); // ❌ CRASH: Compile error! "value borrowed here after move"
    println!("{}", s2);    // ✓ Works perfectly. s2 is the sole owner.
}

```

*Note: Simple types that live entirely on the **Stack** (like integers `i32` or booleans `bool`) implement a trait called `Copy`. Assigning `y = x` on an integer copies the data rather than moving it, because copying a few bytes on the stack is incredibly cheap.*

---

## 🏎️ Borrowing and References (`&`)

Moving ownership every time you want to pass data to a function is annoying. To solve this, Rust uses **Borrowing**. Instead of passing the data itself, you pass a reference (`&`).

Think of it like sharing a file:

* **Move:** Giving someone the original flash drive. You no longer have it.
* **Borrow:** Sending someone a view-only link to your cloud document.

```rust
fn main() {
    let s1 = String::from("actix-web");
    
    // Pass a reference using `&`. We are borrowing s1, not moving it.
    let len = calculate_length(&s1); 

    println!("The length of '{}' is {}.", s1, len); // ✓ s1 is still valid here!
}

fn calculate_length(s: &String) -> usize { // s is a reference to a String
    s.len()
} // s goes out of scope, but because it doesn't OWN the data, nothing happens to the string.

```

### The Rules of Borrowing (The Golden Rule)

To prevent data races (multiple threads or tasks trying to read/write the same memory at once), Rust enforces a strict borrowing rule at compile time:

> You can have **any number of immutable references (`&T`)** OR you can have **exactly one mutable reference (`&mut T`)**, but you can never have both at the same time in the same scope.

```rust
fn main() {
    let mut data = String::from("Database Connection");

    let r1 = &data; // Fine
    let r2 = &data; // Fine (many readers are allowed)
    
    // let r3 = &mut data; // ❌ CRASH: Compile error! Cannot borrow as mutable while it is borrowed as immutable.
    
    println!("{}, {}", r1, r2);
}

```

---

## 💻 Day 2 Practical Exercise: Fixing the Borrow Checker

To truly master ownership, you have to learn how to read compiler errors and satisfy the "Borrow Checker".

### Step 1: Initialize your project

```bash
cargo new day2_borrowing
cd day2_borrowing

```

### Step 2: Replace `src/main.rs`

Copy this broken program into your project. Try to read through it and find the two places where ownership laws are broken before you compile it.

```rust
// This code is intentionally BROKEN. Your job is to fix it!

struct AppConfig {
    server_name: String,
    port: u32,
}

fn main() {
    let config = AppConfig {
        server_name: String::from("Actix-Production-Worker"),
        port: 8080,
    };

    // ❌ PROBLEM 1: This function takes OWNERSHIP of config.
    print_config(config); 

    // ❌ PROBLEM 2: This function tries to change the port, but config wasn't made mutable, 
    // and ownership was already lost above anyway!
    update_port(config, 9000); 

    println!("Server running on port: {}", config.port);
}

fn print_config(cfg: AppConfig) {
    println!("Server: {} running on port {}", cfg.server_name, cfg.port);
}

fn update_port(cfg: AppConfig, new_port: u32) {
    // cfg.port = new_port;
    println!("Updated port to {}", new_port);
}

```

### Step 3: Compile and Debug

Run `cargo check` or `cargo run` in your terminal. Look closely at the error output. The Rust compiler will tell you exactly *where* the data was moved and *why* it can't be used again.

### 🎯 The Fix Strategy

To fix the application so it compiles and prints successfully:

1. Make the initial `config` variable mutable using the `mut` keyword.
2. Change `print_config` to accept a reference (`&AppConfig`) instead of taking ownership, and update the function call to match (`&config`).
3. Change `update_port` to accept a mutable reference (`&mut AppConfig`) and update the call site to match (`&mut config`).

---

## 🎯 Today's Mental Checklist

1. Do I know whether my function is taking **ownership** of a variable or just **borrowing** it?
2. If my Actix web server has 4 worker threads trying to read a configuration string, why does Rust require them to use immutable references (`&`)?
3. What is the fundamental difference between Python assigning a dictionary to a new variable name vs. Rust moving a struct?
