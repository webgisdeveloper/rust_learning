use std::io::{self, Write}; // Import standard input/output library

fn main() {
    println!("--- Welcome to the Day 1 Actix-Preparatory CLI Tool ---");
    println!("Available commands: 'start', 'status', 'help', 'exit'\n");

    loop {
	// 1. Create a mutable, empty String on the Heap to store input
	let mut input = String::new();

	// 2. Read lines from the terminal into our mutable string
	print!("admin@rust-api> ");
	io::stdout().flush().unwrap(); // Force the prompt to print immediately

	io::stdin()
	    .read_line(&mut input)
	    .expect("Failed to read line");

	// 3. CLean up whitespace/newlines from user hitting Enter
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
		println!("💡 Available commands: start, status, help, exit");
		break; // Break out of the loop
	    }
	    // Catch-all for any unrecognized command
	    unknown => {
		println!("❌ Command '{}' not recognized. Type 'help'.", unknown);
	    }
	}
	println!(); // Add a blank line for readability
    }
}
