use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// Model the database row state
#[derive(Clone, Serialize, Deserialize)]
struct Book {
    id: usize,
    title: String,
    author: String,
}

// Define the shared application state container
// In a production app, this would wrap an sqlx:PgPool direct
struct AppState {
    app_name: String,
    mock_db: Mutex<Vec<Book>>, // Mutex allows safe, mutable access threads
}

#[derive(Deserialize)]
struct NewBoookRequest {
    title: String,
    author: String,
}

// Handler: Get /books - extracts the shared state using web::Data
#[get("/books")]
async fn list_books(data: web::Data<AppState>) -> impl Responder {
    // Lock the mutex safely across the async await point to read thedatabse rows
    let books = data.mock_db.lock().await;

    println!("📡 [{}] Fetching all books from mock DB pool...", data.app_name);
    HttpResponse::Ok().json(&*books)
}


// Handler: Post /books - extract both state and a json payload
#[post("/books")]
async fn create_book(data: web::Data<AppState>, payload: web::Json<NewBoookRequest>, ) -> impl Responder {
    let mut books = data.mock_db.lock().await;
    let new_id = books.len() + 1;

    // Why clone here instead of using payload.title directly?
    // `payload` is `web::Json<NewBoookRequest>`, and `title`/`author` are owned `String`s.
    // Writing `payload.title` would try to move the `String` out of `payload`,
    // which is not allowed in this context because `String` is not `Copy`.
    // `clone()` creates a new owned `String` for `new_book` and keeps `payload` intact.
    //
    // Alternative (no clone):
    // let payload = payload.into_inner();
    // title: payload.title,
    // author: payload.author,
    let new_book = Book {
	id: new_id,
	title: payload.title.clone(),
	author: payload.author.clone(),
    };

    books.push(new_book.clone());
    println!("💾 [{}] Inserted book ID: {} successfully.", data.app_name, new_id);

    HttpResponse::Created().json(new_book)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the state before staring http server
    let shared_state = web::Data::new(AppState {
	app_name: String::from("Actix-SQLx-Demo"),
	mock_db: Mutex::new(vec![
	    Book {id: 1, title: String::from("The Rust Programming"), author: String::from("John Ham")}]),
    });

    println!("🚀 Server running at http://127.0.0.1:8080");

    HttpServer::new(move || {
	App::new()
	    .app_data(shared_state.clone())
	    .service(list_books)
	    .service(create_book)
    })
	.bind(("127.0.0.1", 8080))?
	.run()
	.await
}
