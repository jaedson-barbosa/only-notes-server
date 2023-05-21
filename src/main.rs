use axum::{
    extract::{Query, State},
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method, StatusCode,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::prelude::*;
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let _ = dotenv();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let app = create_router(Arc::new(AppState { db: pool.clone() })).layer(cors);

    println!("🚀 Server started successfully");
    axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct AppState {
    db: Pool<Postgres>,
}

#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
struct Note {
    author: String,
    content: String,
    date: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct GetNotes {
    author: String,
    from: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct PostNote {
    author: String,
    content: String,
}

fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/notes", get(get_notes_handler))
        .route("/api/notes", post(post_note_handler))
        .with_state(app_state)
}

async fn get_notes_handler(
    State(data): State<Arc<AppState>>,
    get_params: Query<GetNotes>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let notes = (match get_params.from {
        Some(from) => {
            sqlx::query_as!(
                Note,
                "SELECT * FROM notes WHERE author = $1 and date > $2",
                get_params.author,
                from
            )
            .fetch_all(&data.db)
            .await
        }
        None => {
            sqlx::query_as!(
                Note,
                "SELECT * FROM notes WHERE author = $1",
                get_params.author
            )
            .fetch_all(&data.db)
            .await
        }
    })
    .map_err(|e| {
        let error_response = serde_json::json!({
            "status": "error",
            "message": format!("Database error: {}", e),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
    })?;
    Ok(Json(notes))
}

async fn post_note_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<PostNote>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let new_note = sqlx::query_as!(
        Note,
        "INSERT INTO notes (author,content) VALUES ($1, $2) RETURNING *",
        body.author,
        body.content,
    )
    .fetch_one(&data.db)
    .await
    .map_err(|e| {
        let error_response = serde_json::json!({
            "status": "fail",
            "message": format!("Database error: {}", e),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
    })?;
    Ok(Json(new_note))
}
