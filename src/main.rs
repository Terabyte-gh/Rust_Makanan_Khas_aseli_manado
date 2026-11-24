use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};

#[derive(Serialize, FromRow)]
struct Produk {
    id_makanan: i64,
    makanan: String,
    harga: i64,
    stok: i64,
    deskripsi: String,
}

async fn status() -> Json<Value> {
    Json(json!({
        "status": "success",
        "message": "Server berjalan dengan baik",
        "app": "Makanan Khas Aseli Manado"
    }))
}

async fn get_produk(State(db): State<PgPool>) -> Result<Json<Vec<Produk>>, StatusCode> {
    let produk = sqlx::query_as::<_, Produk>(
        "SELECT id_makanan::bigint AS id_makanan, makanan,
                harga::bigint AS harga, stok::bigint AS stok, deskripsi
         FROM makanan ORDER BY id_makanan",
    )
    .fetch_all(&db)
    .await
    .map_err(|e| {
        eprintln!("[error] {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(produk))
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL belum di-set");

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Gagal konek ke PostgreSQL");

    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/produk", get(get_produk))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3777").await.unwrap();
    println!("Server jalan di http://localhost:3777");
    axum::serve(listener, app).await.unwrap();
}