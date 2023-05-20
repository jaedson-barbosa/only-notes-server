use chrono::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GetNotes {
    pub from: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct PostNote {
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginUser {
    pub email: String,
    pub password: String,
}
