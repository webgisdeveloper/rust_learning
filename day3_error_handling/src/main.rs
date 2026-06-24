/*
Option enum: handling absence
enum Option<T> {
    Some(T), // Contains the value
    None,    // Contains absolutely nothing
}
Result enum: robust error handling
enum Result<T, E> {
    Ok(T),  // The operation succeeded; holds the successful data
    Err(E), // The operation failed; holds the error details
}
 */

// A mock representation of a web request payload
struct ApiRequest {
    body: Option<String>,
    auth_token: Option<String>,
}

#[derive(Debug)]
enum ApiError{
    MissingBody,
    InvalidToken,
}

#[derive(Debug)]
struct AuthenticatedUser {
    username: String,
}

fn main() {
    // Scenario A: A valid incoming request
    let good_request = ApiRequest {
	body: Some(String::from("{'item': 'laptop'}")),
	auth_token: Some(String::from("valid_secret_jwt")),
    };

    // Scenario B: An unauthorized request missing anauth token
    let bad_request = ApiRequest {
        body: Some(String::from("{'item': 'phone'}")),
        auth_token: None,
    };

    println!("--- Processing Scenario A ---");
    match handle_request(good_request) {
	Ok(user) => println!("✓ Request Success! Logged in as: {:?}", user.username),
	Err(err) => println!("❌ Request Failed with error: {:?}", err),
    }

    println!("\n--- Processing Scenario B ---");
    match handle_request(bad_request) {
        Ok(user) => println!("✓ Request Success! Logged in as: {:?}", user.username),
        Err(err) => println!("❌ Request Failed with error: {:?}", err),
    }
}

// A function that authenticates a request, returning a Result
fn handle_request(req: ApiRequest) -> Result<AuthenticatedUser, ApiError> {
    // 1. Unwrapping the auth_token Option
    let token = match req.auth_token {
	Some(t) => t,
	None => return Err(ApiError::InvalidToken),
    };

    // 2. Unwrapping the body Option
    if req.body.is_none() {
	return Err(ApiError::MissingBody);
    }

    // 3. Mock verification logic
    if token == "valid_secret_jwt" {
	Ok(AuthenticatedUser {
	    username: String::from("Dev_John"),
	})
    } else{
	Err(ApiError::InvalidToken)
    }
}

