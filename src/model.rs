use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub password: String,
    pub date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct Note {
    pub id: i32,
    pub author: i32,
    pub content: String,
    pub tags: Vec<String>,
    pub date: Option<DateTime<Utc>>,
}

