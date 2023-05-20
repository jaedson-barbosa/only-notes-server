use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    extract::{Query, State},
    http::{header, Response, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use rand_core::OsRng;
use serde_json::json;

use crate::{
    config::JWT_SECRET, jwt_auth::TokenClaims, model::*, request::*, response::*, AppState,
};

pub async fn login_user_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<LoginUser>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_register = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1",
        body.email.to_ascii_lowercase()
    )
    .fetch_optional(&data.db)
    .await
    .map_err(|e| {
        let error_response = serde_json::json!({
            "status": "error",
            "message": format!("Database error: {}", e),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
    })?;

    let user = match user_register {
        Some(user) => {
            let is_valid = match PasswordHash::new(&user.password) {
                Ok(parsed_hash) => Argon2::default()
                    .verify_password(body.password.as_bytes(), &parsed_hash)
                    .map_or(false, |_| true),
                Err(_) => false,
            };

            if !is_valid {
                let error_response = serde_json::json!({
                    "status": "fail",
                    "message": "Invalid email or password"
                });
                return Err((StatusCode::BAD_REQUEST, Json(error_response)));
            }
            user
        }
        None => {
            let salt = SaltString::generate(&mut OsRng);
            let hashed_password = Argon2::default()
                .hash_password(body.password.as_bytes(), &salt)
                .map_err(|e| {
                    let error_response = serde_json::json!({
                        "status": "fail",
                        "message": format!("Error while hashing password: {}", e),
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
                })
                .map(|hash| hash.to_string())?;

            sqlx::query_as!(
                User,
                "INSERT INTO users (email,password) VALUES ($1, $2) RETURNING *",
                body.email.to_string().to_ascii_lowercase(),
                hashed_password
            )
            .fetch_one(&data.db)
            .await
            .map_err(|e| {
                let error_response = serde_json::json!({
                    "status": "fail",
                    "message": format!("Database error: {}", e),
                });
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
            })?
        }
    };

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::weeks(4)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user.id,
        email: user.email.to_string(),
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .unwrap();

    let cookie = Cookie::build("token", token.to_owned())
        .path("/")
        .max_age(time::Duration::weeks(4))
        .same_site(SameSite::Lax)
        .http_only(true)
        .finish();

    let mut response = Response::new(token);
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());
    Ok(response)
}

pub async fn logout_handler() -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cookie = Cookie::build("token", "")
        .path("/")
        .max_age(time::Duration::hours(-1))
        .same_site(SameSite::Lax)
        .http_only(true)
        .finish();

    let mut response = Response::new(json!({"status": "success"}).to_string());
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());
    Ok(response)
}

pub async fn get_notes_handler(
    State(data): State<Arc<AppState>>,
    Extension(token): Extension<TokenClaims>,
    pagination: Query<GetNotes>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let notes = (match pagination.from {
        Some(from) => {
            sqlx::query_as!(
                Note,
                "SELECT * FROM notes WHERE id = $1 and date > $2",
                token.sub,
                from
            )
            .fetch_all(&data.db)
            .await
        }
        None => {
            sqlx::query_as!(Note, "SELECT * FROM notes WHERE id = $1", token.sub)
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
    let response = NotesResponse {
        author: token.email.to_owned(),
        notes: notes.iter().map(|v| FilteredNote::from(v)).collect(),
    };
    Ok(Json(response))
}

pub async fn post_note_handler(
    State(data): State<Arc<AppState>>,
    Extension(token): Extension<TokenClaims>,
    Json(body): Json<PostNote>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let new_note = sqlx::query_as!(
        Note,
        "INSERT INTO notes (author,content,tags) VALUES ($1, $2, $3) RETURNING *",
        token.sub,
        body.content,
        &body.tags
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
    let filtered = FilteredNote::from(&new_note);
    Ok(Json(filtered))
}
