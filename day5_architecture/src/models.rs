// Define the struct and mark it as public
#[derive(Debug)]
pub struct UserProfile {
    pub username: String,
    pub role: String,
    salary: f64,
}

impl UserProfile {
    pub fn new(username: String, role: String, salary: f64) -> Self {
        Self { username, role, salary }
    }
}
