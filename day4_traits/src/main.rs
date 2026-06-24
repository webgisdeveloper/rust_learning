// Define the mock "HTTP Request"
struct HttpRequest {
    payload: String,
}

// Define the extractor trait
trait FromRequest {
    // `where Self: Sized` is required because this method returns `Self` by value
    // inside a Result<Self, ...>. Rust needs to know the exact size of `Self` at
    // compile time to allocate it on the stack. Trait objects (dyn FromRequest)
    // are NOT Sized — they are just pointers to unknown types — so this bound
    // restricts the method to concrete, fully-known types only (e.g. UserIdExtractor).
    // Without this bound, Rust would reject the return type `Result<Self, String>`
    // because it cannot determine how much memory `Self` needs.
    fn extract(req: &HttpRequest) -> Result<Self, String> where Self: Sized;
}

// Define two different structs
struct UserIdExtractor(u64);
struct SearchQueryExtractor(String);

// Implement the trait for the User ID extractor
impl FromRequest for UserIdExtractor {
    fn extract(req: &HttpRequest) -> Result<Self, String> {
	match req.payload.trim().parse::<u64>() {
	    Ok(id) => Ok(UserIdExtractor(id)),
	    Err(_) => Err(String::from("400 Bad Request: Invalid User ID numeric format")),
	}
    }
}

// Implement the trait for SerchQueryExtractor
impl FromRequest for SearchQueryExtractor {
    fn extract(req: &HttpRequest) -> Result<Self, String> {
	if req.payload.is_empty() {
	    Err(String::from("400 Bad Request: Search query cannot be empty"))
	} else {
	    Ok(SearchQueryExtractor(req.payload.clone()))
	}
    }
}

// A generic route handler for any exactor type T
fn extract_and_process<T: FromRequest>(req: HttpRequest, handler: fn(T)) {
    match T::extract(&req) {
	Ok(extract_data) => handler(extract_data),
	Err(err_msg) => println!("❌ Extractor Failed: {}", err_msg),
    }
}

fn main() {
    // Scenario A: a request hitting a GET /user/{id} endpoint
    let req_a = HttpRequest {payload: String::from("10523")};
    println!("--- Routing Request A (User ID) ---");

    // A closure is an anonymous (nameless) function defined inline with |params| { body }.
    // Here `|data|` receives a `UserIdExtractor` value (inferred from the turbofish above).
    // The closure is passed as the `handler` argument and acts as a callback:
    // it is stored inside `extract_and_process` and only called if extraction succeeds (Ok branch).
    // `data.0` accesses the first field of the UserIdExtractor tuple struct — the extracted u64 ID.
    extract_and_process::<UserIdExtractor>(req_a, |data| {
	println!("✓ Success! Routing to dashboard for User ID: {}", data.0);
    });

    // Scenario B: A request hitting a search endpoint with invalid data
    let req_b = HttpRequest { payload: String::from("")};
    println!("\n--- Routing Request B (search query) ---");

    // A different closure is passed here for a different success action.
    // Because the payload is empty, extraction will fail and this closure will NOT be called.
    // Instead, extract_and_process will print the Err message from SearchQueryExtractor::extract.
    // This shows how the same generic function can serve different routes with different callbacks.
    extract_and_process::<SearchQueryExtractor>(req_b,|data| {
	println!("✓ Success! Performing database search for: {}", data.0);
    });
    
}
