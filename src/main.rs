use axum::{
    extract::{Query, State},
    http::{Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::prelude::*;
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tower_http::cors::{Any,CorsLayer};

#[tokio::main]
async fn main() {
    let _ = dotenv();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
    let port = std::env::var("PORT")
        .expect("PORT must be set.")
        .parse::<u16>()
        .expect("PORT must be a valid number.");
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
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let app = create_router(Arc::new(AppState { db: pool.clone() })).layer(cors);

    println!("🚀 Server started successfully");
    let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    axum::Server::bind(&addr)
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
    iv: String,
    content: String,
    date: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct Date {
    date: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct GetNotes {
    author: String,
    from: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct CheckRegister {
    author: String,
}

#[derive(Serialize)]
struct CheckRegisterResponse {
    registered: bool,
    date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct PostNote {
    author: String,
    content: String,
    iv: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/account", get(check_account))
        .route("/notes", get(get_notes_handler))
        .route("/notes", post(post_note_handler))
        .with_state(app_state)
}

async fn check_account(
    State(data): State<Arc<AppState>>,
    get_params: Query<CheckRegister>,
) -> Result<Json<CheckRegisterResponse>, (StatusCode, Json<ErrorResponse>)> {
    let first = sqlx::query_as!(
        Date,
        "SELECT date FROM notes WHERE author = $1 LIMIT 1",
        get_params.author
    )
    .fetch_optional(&data.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: format!("Database error: {}", e),
            }),
        )
    })?;
    Ok(Json(match first {
        Some(first) => CheckRegisterResponse {
            registered: true,
            date: Some(first.date),
        },
        None => CheckRegisterResponse {
            registered: false,
            date: None,
        },
    }))
}

async fn get_notes_handler(
    State(data): State<Arc<AppState>>,
    get_params: Query<GetNotes>,
) -> Result<Json<Vec<Note>>, (StatusCode, Json<ErrorResponse>)> {
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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: format!("Database error: {}", e),
            }),
        )
    })?;
    Ok(Json(notes))
}

async fn post_note_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<PostNote>,
) -> Result<Json<Note>, (StatusCode, Json<ErrorResponse>)> {
    let new_note = sqlx::query_as!(
        Note,
        "INSERT INTO notes (author,content,iv) VALUES ($1, $2, $3) RETURNING *",
        body.author,
        body.content,
        body.iv
    )
    .fetch_one(&data.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: format!("Database error: {}", e),
            }),
        )
    })?;
    Ok(Json(new_note))
}
