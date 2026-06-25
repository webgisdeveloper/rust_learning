use actix_web::{get, App, HttpResponse, HttpServer, Responder};

// Define an asynchronous route handler using get macro
#[get("/")]
async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello from your multi-threaded Actix Web server!")
}

// Define a second async router handler
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("{\"status\": \"healthy\"}")
}

// Set up the async runtime engine under main function
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Launching Actix Web server on http://127.0.0.1:8080");

    // Instantiated the multi-threaded server
    HttpServer::new(|| {
	// This closure runs for every thread/cpu worker core Actix creats
	App::new()
	    .service(hello_world)  // Register / route
	    .service(health_check) // Register /helath route
    })
	.bind(("127.0.0.1", 8080))? // Bind to local host on port 8080 
	.run() // Run the execution engine loop
	.await // Wait asynchronously for shutdown signals
}
