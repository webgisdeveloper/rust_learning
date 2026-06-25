// Explicitly declare top-level modules in mains.rs
mod models;
mod routes;

fn main() {
    println!("--- Initializing Modular Project Architecture ---");

    // Call the nested module function
    routes::user_routes::mock_create_user_handleer();
}
