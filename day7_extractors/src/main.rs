use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

// Model the structure of an incoming JSON body for creating items
#[derive(Deserialize, Debug)]
struct CreateProduct{
    name: String,
    price: f64,
    inventory: u32,
}

// Model the structure of an incoming URL query paramter
#[derive(Deserialize, Debug)]
struct ProductFilter {
    search: Option<String>,
    limit: Option<usize>,
}

// Model the structure of the JSON response
#[derive(Serialize)]
struct ProductReponse {
    id: u64,
    name: String,
    price: f64,
}

// Handler: POST /products (extract JSON payload
#[post("/products")]
async fn create_product(payload: web::Json<CreateProduct>) -> impl Responder {
    println!("📥 Received request to create product: {:?}", payload);

    // Create a mock response object
    let reponse = ProductReponse {
	id: 991,
	name: payload.name.clone(),
	price: payload.price,
    };

    HttpResponse::Created().json(reponse)
}

#[get("/products/{id}")]
async fn get_product_by_id(path: web::Path<u64>) -> impl Responder {
    let product_id = path.into_inner(); // Unwraps the inner u64
    println!("🔍 Searching for product ID: {}", product_id);

    if product_id == 404 {
	return HttpResponse::NotFound().body("Product not found");
    }

    let response = ProductReponse {
	id: product_id,
	name: String::from("High-Performance Ruse Textbook"),
	price: 49.99,
    };
    
    HttpResponse::Ok().json(response)
}

// Handler: Get /products (extract url query params)
#[get("/products")]
async fn list_products(query: web::Query<ProductFilter>) -> impl Responder {
    println!("📋 Listing products with filters applied: {:?}", query);
    HttpResponse::Ok().body("Product filter queries executed successfully.")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting extractor server on http://127.0.0.1:8080");

    HttpServer::new (|| {
	App::new()
	    .service(create_product)
	    .service(get_product_by_id)
	    .service(list_products)
    })
	.bind(("127.0.0.1", 8080))?
	.run()
	.await
}
