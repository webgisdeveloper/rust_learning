# Mastodon CLI Tool

A simple Rust-based command-line interface (CLI) tool to post status updates to [mastodon.social](https://mastodon.social). This project is designed as a learning example for beginners to see how standard Rust libraries are used for CLI development and HTTP requests.

## Features

- Parse command-line arguments using `clap`.
- Handle asynchronous HTTP requests with `reqwest` and `tokio`.
- Serialize data to JSON using `serde`.
- Support for both command-line flags and environment variables for authentication.
- Automatic replacement of emoji shortcodes (e.g., :apple: -> 🍎).
- Cleans HTML tags and decodes entities from fetched statuses for a clean CLI output.

## Prerequisites

You must have the Rust toolchain installed. If you don't have it, you can install it via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

1. Clone the repository or navigate to the project directory.
2. Build the project:

```bash
cargo build --release
```

The binary will be located at `target/release/mastodon_cli`.

## Configuration

To avoid passing your access token every time, you can set it as an environment variable:

```bash
export MASTODON_TOKEN=your_access_token_here
```

## Usage

### Basic Usage (using environment variable)
If `MASTODON_TOKEN` is set:
```bash
cargo run -- --message "Hello from my Rust CLI! :rocket:"
```

### Usage with explicit token
```bash
cargo run -- --message "I am eating an :apple: :smile:" --token your_access_token_here
```

### Available Flags
- `-m, --message <TEXT>`: The status message to post (Required).
- `-t, --token <TOKEN>`: The Mastodon API access token (Optional if `MASTODON_TOKEN` is set).
- `-h, --help`: Print help message.
- `-V, --version`: Print version information.

## Technical Overview

This tool uses the following industry-standard Rust crates:

- **`clap`**: Handles CLI argument parsing with a declarative derive-macro approach.
- **`reqwest`**: An async HTTP client used to communicate with the Mastodon API.
- **`tokio`**: The async runtime that executes the `reqwest` futures.
- **`serde`**: The serialization framework used to turn Rust structs into JSON payloads.
- **`html-escape`**: Used to decode HTML entities (like `&gt;` to `>`) in retrieved status content.
