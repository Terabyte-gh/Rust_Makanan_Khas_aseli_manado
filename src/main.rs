use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

async fn status() -> Json<Value> {
    Json(json!({
        "status": "success",
        "message": "Server berjalan dengan baik",
        "app": "Makanan Khas Aseli Manado"
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/status", get(status));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3777").await.unwrap();
    println!("Server jalan di http://localhost:3777");
    axum::serve(listener, app).await.unwrap();
}