# checkIP_cli

A simple command-line tool written in Rust that displays your system's hostname, local IP address, and public IP address.

## Features

- **Hostname**: Retrieves your system's hostname
- **Local IP Address**: Determines the local IP address used for external connections
- **Public IP Address**: Fetches your public IP address as seen from the internet

## Prerequisites

- Rust (2024 edition or later)
- Cargo package manager

## Installation

Clone the repository and build the project:

```bash
git clone <repository-url>
cd checkIP_cli
cargo build --release
```

## Usage

Run the application:

```bash
cargo run
```

Or run the compiled binary:

```bash
./target/release/checkIP_cli
```

### Example Output

```
Hostname: my-computer
Local IP Address: 192.168.1.100
Public IP Address: 203.0.113.45
```

## How It Works

- **Hostname**: Executes the system's `hostname` command
- **Local IP**: Creates a UDP socket connection to 8.8.8.8 (no data is sent) to determine the local interface IP
- **Public IP**: Queries the [ipify API](https://www.ipify.org/) to retrieve your public-facing IP address

## Dependencies

- `reqwest` (v0.12.24) - HTTP client for fetching the public IP address

## License

This project is open source and available for educational purposes.
