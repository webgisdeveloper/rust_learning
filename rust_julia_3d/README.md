# Rust + Julia 3D Surface Visualization (`rust_julia_3d`)

A Rust application demonstrating interoperability with the Julia programming language using the [`jlrs`](https://crates.io/crates/jlrs) crate. The application embeds the Julia runtime into Rust, automatically ensures the `GLMakie` package is installed in Julia, generates 3D surface plot data ($f(x, y) = \sin(\sqrt{x^2 + y^2})$), and renders an interactive 3D window.

---

## Technical Stack

- **Rust**: 2024 Edition
- **Julia Interoperability**: `jlrs` v0.24 (with `local-rt` feature enabled)
- **Julia Plotting Library**: `GLMakie` (via Julia `Pkg` dynamic package resolution)

---

## Prerequisites

1. **Rust Toolchain**: `cargo` and `rustc` installed (e.g., via [`rustup`](https://rustup.rs/)).
2. **Julia**: Julia installed and available in your `PATH` (e.g., via [`juliaup`](https://github.com/JuliaLang/juliaup)).

---

## Environment Setup (`.bashrc` Export Commands)

To compile Rust applications using `jlrs` and allow the Linux dynamic library loader to locate Julia's shared libraries (`libjulia.so`) at build time and runtime, the following environment variables must be configured in your shell environment (`~/.bashrc`).

### Added `.bashrc` Configuration

Add the following export snippet to your `~/.bashrc`:

```bash
# Setup environment paths for jlrs (Rust + Julia integration)
if command -v julia &>/dev/null; then
    export JLRS_JULIA_DIR="$(julia -e 'print(dirname(Sys.BINDIR))' 2>/dev/null)"
    export LD_LIBRARY_PATH="$JLRS_JULIA_DIR/lib:${LD_LIBRARY_PATH:-}"
fi
```

### Explanation of Environment Variables

- **`JLRS_JULIA_DIR`**: Root directory of the active Julia installation. Used by `jl-sys` during compilation to locate Julia C headers and libraries.
- **`LD_LIBRARY_PATH`**: Appends `$JLRS_JULIA_DIR/lib` to the linker path so executable binaries can dynamically link `libjulia.so` at runtime.

### Apply Configuration

To apply these changes to your current terminal session:

```bash
source ~/.bashrc
```

---

## Building and Running

1. **Build the Project**:
   ```bash
   cargo build
   ```

2. **Run the Application**:
   ```bash
   cargo run
   ```

Upon execution:
- Julia will initialize inside the Rust runtime thread.
- If `GLMakie` is not installed in your Julia environment, it will automatically download and install it.
- A 3D surface plot window will render interactively.
- Press <kbd>Enter</kbd> in your terminal to exit the application.

---

## Project Structure

```
rust_julia_3d/
├── Cargo.toml      # Dependency declaration (jlrs with local-rt)
├── README.md       # Project documentation & environment setup
└── src/
    └── main.rs     # Rust main function embedding & executing Julia code
```
