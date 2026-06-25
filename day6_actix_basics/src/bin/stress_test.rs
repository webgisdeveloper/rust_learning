use futures::stream::{self, StreamExt};
use reqwest::Client;
use std::time::Instant;

const URL: &str = "http://127.0.0.1:8080/health";
const TOTAL_REQUESTS: usize = 10_000;
const CONCURRENCY: usize = 200;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let start = Instant::now();

    let results: Vec<Result<u16, reqwest::Error>> = stream::iter(0..TOTAL_REQUESTS)
        .map(|_| {
            let client = client.clone();
            async move {
                client
                    .get(URL)
                    .send()
                    .await
                    .map(|resp| resp.status().as_u16())
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let elapsed = start.elapsed().as_secs_f64();
    let success = results.iter().filter(|r| matches!(r, Ok(200))).count();
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
