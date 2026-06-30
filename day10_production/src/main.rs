use actix_web::{get, App, HttpResponse, HttpServer, Responder};

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()>{
    println!("🚀 Starting production target on http://127.0.0.1:8080");
    HttpServer::new(|| {
	App::new()
	.service(health_check)
    })
	.bind(("127.0.0.1", 8080))?
	.run()
	.await
}

// Testing suite
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_health_endpoint_returns_200() {
	let app = test::init_service(App::new().service(health_check)).await;
	let req = test::TestRequest::get().uri("/health").to_request();
	let resp = test::call_service(&app, req).await;

	assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

	let body = test::read_body(resp).await;
	assert_eq!(body, actix_web::web::Bytes::from_static(b"OK"));
	    
    }
}
