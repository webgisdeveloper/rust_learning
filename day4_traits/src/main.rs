// Define the mock "HTTP Request"
struct HttpRequest {
    payload: String,
}

// Define the extractor trait
trait FromRequest {
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
    extract_and_process::<UserIdExtractor>(req_a, |data| {
	println!("✓ Success! Routing to dashboard for User ID: {}", data.0);
    });

    // Scenario B: A request hitting a search endpoint with invalid data
    let req_b = HttpRequest { payload: String::from("")};
    println!("\n--- Routing Request B (search query) ---");
    extract_and_process::<SearchQueryExtractor>(req_b,|data| {
	println!("✓ Success! Performing database search for: {}", data.0);
    });
    
}
