;; This buffer is for text that is not saved, and for Lisp evaluation.
;; To create a file, visit it with ‘C-x C-f’ and enter text in its buffer.

# Day 1: The Rust Mental Model & Syntax Basics

Welcome to Day 1! Coming from Python, your biggest hurdle today won't be syntax—it will be adjusting to **how** Rust thinks about data, memory, and code safety.

Python abstracts away the machine to optimize for developer speed. Rust exposes the machine to optimize for runtime speed and safety, using a strict compiler to guarantee that your code won't crash in production due to memory issues.

---

## 🧠 The "Py-to-Rust" Mental Model

Before writing code, let's look at the foundational differences in how these two languages operate:

| Concept | Python | Rust |
| --- | --- | --- |
| **Execution** | Interpreted / JIT compiled (Bytecode at runtime) | Ahead-of-Time (AOT) compiled to a native machine binary |
| **Memory Management** | Automatic Garbage Collector (Reference Counting + Generational GC) | Compile-time ownership (Zero runtime GC overhead) |
| **Variables** | Dynamically typed labels pointing to objects on the Heap | Statically typed, bound to specific memory slots (Stack by default) |
| **Mutability** | Most objects are mutable unless explicitly immutable (e.g., tuples) | **Immutable by default**. You must explicitly ask for mutability. |
| **Errors** | Runtime Exceptions (`try/except`) | Compile-time checks & explicit value wrapping (`Result`/`Option`) |

---

## 🛠️ Core Syntax Basics

### 1. Variables & Mutability

In Python, you can reassign anything at any time. In Rust, variables are locked down by default.

```python
# Python
x = 5
x = 6 # Perfectly fine

```

```rust
// Rust
fn main() {
    let x = 5;
    // x = 6; // ❌ CRASH: Compile-time error! "cannot assign twice to immutable variable"

    let mut y = 5;
    y = 6; //  Allowed because of the `mut` keyword
}

```

### 2. The Stack vs. The Heap

* **The Stack:** Fast, fixed-size memory allocation. Local variables like integers (`i32`), booleans (`bool`), and fixed-size arrays live here.
* **The Heap:** Slower, dynamic memory allocation. Things that can grow at runtime (like dynamically sized `String` types or Vectors) live here.

Python puts almost everything on the heap and passes around references. Rust forces you to care. If a size is known at compile time, it goes on the quick stack.

### 3. Control Flow & Pattern Matching

Python recently introduced `match/case`, but Rust’s `match` statement is a foundational language pillar. It is **exhaustive**, meaning the compiler will refuse to run your code if you miss a single possible condition.

```rust
fn main() {
    let status_code = 404;

    match status_code {
        200 => println!("Success!"),
        400 | 401 => println!("Client Error"),
        404 => println!("Not Found!"),
        _ => println!("Something else"), // The `_` acts as a catch-all (like Python's `case _`)
    }
}

```

---

## 💻 Day 1 Practical Exercise: The Interactive CLI Parser

To solidify today's concepts, you are going to build a basic command-line interface (CLI) tool that reads input from the terminal and routes it using pattern matching.

### Step 1: Create a new project

Open your terminal and run:

```bash
cargo new day1_cli
cd day1_cli

```

This sets up a fresh directory with a `src/main.rs` file.

### Step 2: Write the Code

Replace the contents of `src/main.rs` with the following code. Read the comments carefully to see how we handle terminal input and match strings:

```rust
use std::io::{self, Write}; // Import standard input/output library

fn main() {
    println!("--- Welcome to the Day 1 Actix-Preparatory CLI Tool ---");
    println!("Available commands: 'start', 'status', 'help', 'exit'\n");

    loop {
        // 1. Create a mutable, empty String on the Heap to store input
        let mut input = String::new();

        print!("admin@rust-api> ");
        io::stdout().flush().unwrap(); // Force the prompt to print immediately

        // 2. Read lines from the terminal into our mutable string
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        // 3. Clean up whitespace/newlines (\n) from user hitting Enter
        let command = input.trim().to_lowercase();

        // 4. Use pattern matching to route the command
        match command.as_str() {
            "start" => {
                println!("🚀 Starting mock Actix Web server instance...");
            }
            "status" => {
                println!("🟢 System status: Nominal. Thread pool optimized.");
            }
            "help" => {
                println!("💡 Available commands: start, status, help, exit");
            }
            "exit" => {
                println!("👋 Exiting. Happy hacking!");
                break; // Break out of the loop
            }
            // Exhaustive check: Catch-all for any unrecognized command
            unknown => {
                println!("❌ Command '{}' not recognized. Type 'help'.", unknown);
            }
        }
        println!(); // Add a blank line for readability
    }
}

```

### Step 3: Run the Application

In your terminal, execute:

```bash
cargo run

```

Test out the inputs (`start`, `status`, `invalid_command`, `exit`) to observe how pattern matching catches everything seamlessly.

---

## 🎯 Today's Mental Checklist

1. Did I use `mut` whenever I needed to change a variable's value later?
2. Do I understand why `String::new()` creates a string on the **heap** while a number like `5` lives on the **stack**?
3. Did I see how Rust’s compiler protects me from typos by requiring the `_` or `unknown` catch-all branch in the `match` block?
