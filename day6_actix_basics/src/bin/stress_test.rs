use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::time::Instant;

// -------- Config --------
// `const` values are compile-time constants.
// Rust wants explicit types; Python infers them dynamically.
const URL: &str = "http://127.0.0.1:8080/health";
const TOTAL_REQUESTS: usize = 10_000;
const CONCURRENCY: usize = 200;

// This macro sets up the Tokio async runtime for us.
// Python equivalent idea: wrapping async main with `asyncio.run(main())`.
#[tokio::main]
async fn main() {
    // Reusable HTTP client (good practice in both Rust and Python).
    let client = Client::new();

    // Start a timer.
    let start = Instant::now();

    // `Vec<Result<u16, reqwest::Error>>` means:
    // - Vec: growable array (like list)
    // - Result<OkType, ErrType>: success or error, explicit in type system
    // - Ok(u16): HTTP status code
    // - Err(reqwest::Error): network/protocol error
    let results: Vec<Result<u16, reqwest::Error>> = stream::iter(0..TOTAL_REQUESTS)
        // Build one async task per request.
        // `|_|` means "input exists, but I don't care about its value".
        .map(|_| {
            // Rust ownership note:
            // each async task needs access to `client`.
            // Cloning reqwest::Client is cheap (internally shared handle).
            let client = client.clone();

            // `async move`:
            // - `async` => this block returns a Future
            // - `move`  => capture variables by value (task owns what it needs)
            // Think: "freeze needed variables into this coroutine/task".
            async move {
                client
                    .get(URL)
                    .send()
                    .await
                    // If send succeeded, transform Response -> status code.
                    // `.map(...)` here is Result::map, not iterator map.
                    .map(|resp| resp.status().as_u16())
            }
        })
        // Run up to CONCURRENCY futures at once.
        // Similar outcome to Python: semaphore-limited gather.
        .buffer_unordered(CONCURRENCY)
        // Pull everything into memory as a Vec.
        .collect()
        .await;

    let elapsed = start.elapsed().as_secs_f64();

    // Count only exact Ok(200) results.
    // `matches!` is pattern matching shorthand.
    let success = results.iter().filter(|r| matches!(r, Ok(200))).count();

    // Anything not Ok(200) is counted as failed.
    // (Includes Err(...) and non-200 statuses.)
    let failed = TOTAL_REQUESTS - success;

    println!("Stress test complete");
    println!("URL: {}", URL);
    println!("Total requests: {}", TOTAL_REQUESTS);
    println!("Concurrency: {}", CONCURRENCY);
    println!("Successful responses: {}", success);
    println!("Failed responses: {}", failed);
    println!("Elapsed time: {:.2} seconds", elapsed);
    println!("Requests/sec: {:.2}", TOTAL_REQUESTS as f64 / elapsed);
}
