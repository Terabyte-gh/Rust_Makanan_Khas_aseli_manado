use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Datelike;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::sync::Arc;
use tera::{Context, Kwargs, State as TeraState, Tera};

#[derive(Serialize, FromRow)]
struct Produk {
    id_makanan: i64,
    makanan: String,
    harga: i64,
    stok: i64,
    deskripsi: String,
}

async fn status(State(app_name): State<Arc<str>>) -> Json<Value> {
    Json(json!({
        "status": "success",
        "message": "Server berjalan dengan baik",
        "app": app_name.as_ref()
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

#[derive(Clone)]
struct AppState {
    db: PgPool,
    tera: Arc<Tera>,
    app_name: Arc<str>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(s: &AppState) -> Self {
        s.db.clone()
    }
}
impl FromRef<AppState> for Arc<Tera> {
    fn from_ref(s: &AppState) -> Self {
        s.tera.clone()
    }
}
impl FromRef<AppState> for Arc<str> {
    fn from_ref(s: &AppState) -> Self {
        s.app_name.clone()
    }
}

fn rupiah(value: i64, _: Kwargs, _: &TeraState) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let out: String = out.chars().rev().collect();
    format!("Rp {out}")
}

async fn index(
    State(db): State<PgPool>,
    State(tera): State<Arc<Tera>>,
    State(app_name): State<Arc<str>>,
) -> Result<Html<String>, StatusCode> {
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

    let mut ctx = Context::new();
    ctx.insert("app_name", app_name.as_ref());
    ctx.insert("produk", &produk);
    ctx.insert("tahun", &chrono::Utc::now().year());
    ctx.insert("is_admin", &false);

    let html = tera.render("index.html", &ctx).map_err(|e| {
        eprintln!("[error] {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Html(html))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .unwrap_or_else(|e| panic!("{key} tidak valid: {e}")),
        Err(_) => default,
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL belum di-set");
    let app_name: Arc<str> = env_or("APP_NAME", "Makanan Khas Aseli Manado").into();
    let template_glob = env_or("TEMPLATE_GLOB", "templates/**/*");
    let host = env_or("HOST", "0.0.0.0");
    let port: u16 = env_parse("PORT", 3777);
    let max_connections: u32 = env_parse("DB_MAX_CONNECTIONS", 10);

    let db = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await
        .expect("Gagal konek ke PostgreSQL");

    let mut tera = Tera::new();
    tera.register_filter("rupiah", rupiah);
    tera.load_from_glob(&template_glob)
        .expect("Gagal load template");

    let state = AppState {
        db,
        tera: Arc::new(tera),
        app_name,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/status", get(status))
        .route("/api/produk", get(get_produk))
        .route("/api/produk/{id}", get(get_produk_by_id))
        .with_state(state);

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(addr.as_str())
        .await
        .unwrap_or_else(|e| panic!("Gagal bind ke {addr}: {e}"));
    println!("Server jalan di http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn get_produk_by_id(
    State(db): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<Produk>, (StatusCode, Json<Value>)> {
    let produk = sqlx::query_as::<_, Produk>(
        "SELECT id_makanan::bigint AS id_makanan, makanan,
                harga::bigint AS harga, stok::bigint AS stok, deskripsi
         FROM makanan WHERE id_makanan = $1",
    )
    .bind(id)
    .fetch_optional(&db)
    .await
    .map_err(|e| {
        eprintln!("[error] {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": "Terjadi kesalahan pada server"})),
        )
    })?;

    produk.map(Json).ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({"status": "error", "message": "Produk tidak ditemukan"})),
    ))
}


