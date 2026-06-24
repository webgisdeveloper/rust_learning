/*
Why this version?
- Keeps explicit `match` / `if let` so control flow is easy to follow for beginners.
- Adds clearer error messages and validation steps.
- Uses valid JSON formatting in sample request strings.
- Uses references (`&ApiRequest`) so the request is borrowed, not moved.
*/

use std::fmt;

// A mock representation of a web request payload
struct ApiRequest {
    body: Option<String>,
    auth_token: Option<String>,
}

// `Debug` helps during development/logging.
// Added `EmptyBody` so we can distinguish between "missing" and "present but blank".
#[derive(Debug)]
enum ApiError {
    MissingBody,
    EmptyBody,
    InvalidToken,
}

// `Display` gives cleaner, user-friendly output than raw debug text.
impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::MissingBody => write!(f, "Request body is missing."),
            ApiError::EmptyBody => write!(f, "Request body is empty."),
            ApiError::InvalidToken => write!(f, "Authentication token is missing or invalid."),
        }
    }
}

#[derive(Debug)]
struct AuthenticatedUser {
    username: String,
}

fn main() {
    // Scenario A: a valid incoming request
    // Change made: use valid JSON style with double quotes.
    let good_request = ApiRequest {
        body: Some(String::from("{\"item\": \"laptop\"}")),
        auth_token: Some(String::from("valid_secret_jwt")),
    };

    // Scenario B: unauthorized request missing an auth token
    // Change made: fixed typo in comment ("an auth token").
    let bad_request = ApiRequest {
        body: Some(String::from("{\"item\": \"phone\"}")),
        auth_token: None,
    };

    println!("--- Processing Scenario A ---");
    match handle_request(&good_request) {
        Ok(user) => println!("✓ Request success! Logged in as: {}", user.username),
        Err(err) => println!("❌ Request failed: {}", err),
    }

    println!("\n--- Processing Scenario B ---");
    match handle_request(&bad_request) {
        Ok(user) => println!("✓ Request success! Logged in as: {}", user.username),
        Err(err) => println!("❌ Request failed: {}", err),
    }
}

// Change made: take `&ApiRequest` (borrow) instead of owning `ApiRequest`.
// Why: avoids moving request data and is a good habit when you only need to read it.
fn handle_request(req: &ApiRequest) -> Result<AuthenticatedUser, ApiError> {
    // 1) Validate token presence (beginner-friendly explicit match).
    let token = match &req.auth_token {
        Some(t) => t, // borrow token as &String
        None => return Err(ApiError::InvalidToken),
    };

    // 2) Validate body presence (explicit match for clarity).
    let body = match &req.body {
        Some(b) => b, // borrow body as &String
        None => return Err(ApiError::MissingBody),
    };

    // 3) Validate body content is not blank.
    // Why: Some("") should usually be treated as invalid input.
    if body.trim().is_empty() {
        return Err(ApiError::EmptyBody);
    }

    // 4) Mock token verification.
    // Change made: moved verification to helper function for readability.
    if is_valid_token(token) {
        Ok(AuthenticatedUser {
            username: String::from("Dev_John"),
        })
    } else {
        Err(ApiError::InvalidToken)
    }
}

// Small helper function keeps auth logic separate and easy to replace later.
fn is_valid_token(token: &str) -> bool {
    token == "valid_secret_jwt"
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build requests quickly in each test.
    // Why: avoids repeating ApiRequest { ... } boilerplate.
    fn make_request(body: Option<&str>, token: Option<&str>) -> ApiRequest {
        ApiRequest {
            body: body.map(String::from),
            auth_token: token.map(String::from),
        }
    }

    #[test]
    fn handle_request_success_with_valid_token_and_body() {
        let req = make_request(Some("{\"item\": \"laptop\"}"), Some("valid_secret_jwt"));

        let result = handle_request(&req);

        // Beginner-friendly and concise:
        // `expect` fails the test with a clear message if result is Err.
        let user = result.expect("Expected successful authentication");
        assert_eq!(user.username, "Dev_John");
    }

    #[test]
    fn handle_request_fails_when_token_missing() {
        let req = make_request(Some("{\"item\": \"phone\"}"), None);

        let result = handle_request(&req);

        // Compare Result directly.
        assert!(matches!(result, Err(ApiError::InvalidToken)));
    }

    #[test]
    fn handle_request_fails_when_body_missing() {
        let req = make_request(None, Some("valid_secret_jwt"));

        let result = handle_request(&req);

        assert!(matches!(result, Err(ApiError::MissingBody)));
    }

    #[test]
    fn handle_request_fails_when_body_is_empty_or_whitespace() {
        let req = make_request(Some("   "), Some("valid_secret_jwt"));

        let result = handle_request(&req);

        assert!(matches!(result, Err(ApiError::EmptyBody)));
    }

    #[test]
    fn handle_request_fails_when_token_is_wrong_value() {
        let req = make_request(Some("{\"item\": \"tablet\"}"), Some("wrong_secret_jwt"));

        let result = handle_request(&req);

        assert!(matches!(result, Err(ApiError::InvalidToken)));
    }
}
