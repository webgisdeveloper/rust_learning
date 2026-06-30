use actix_web::{
    get, middleware::Logger, web, App, HttpRequest, HttpResponse, HttpServer, Responder
};
use env_logger::{Builder, Env};

#[get("/public")]
async fn public_route() -> impl Responder {
    HttpResponse::Ok().body("🔓 This endpoint is completely public.")
}

#[get("/secret")]
async fn secret_route(req: HttpRequest) -> impl Responder {
    // Inspect the incoming http request headers manuallyq
    match req.headers().get("X-API-Key") {
	Some(header_value) => {
	    // Convert header bytes to a string slice safely
	    if let Ok(token) = header_value.to_str() {
		if token == "super-secret-token-123" {
		    return HttpResponse::Ok().body("👑 Welcome to the secure admin control panel!")};
	    }
	    // Token was present but invalid
	    HttpResponse::Unauthorized().body("❌ Invalid Authorization Token.")
	}
	None => {
	    // Token was completely missing
	    HttpResponse::BadRequest().body("❌ Missing required 'X-API-Key' header.")
 	}
    }
}


#[actix_web::main]
async fn main() ->std::io::Result<()> {
    // Initialize the logger terminal output, similar to logging.basicConfig
    // not safe 
    // std::env::set_var("RUST_LOG", "actix_web=info");
    // env_logger::init();
    Builder::from_env(Env::default().default_filter_or("actix_web=info")).init();
    println!("🚀 Launching Middleware & Security server at http://127.0.0.1:8080");

    HttpServer::new(|| {
	App::new()
	// Register the built-in logger middleware wrap
	    .wrap(Logger::default())
	    .service(public_route)
	    .service(secret_route)
    })
	.bind(("127.0.0.1",8080))?
	.run()
	.await
}
