use dotenvy_macro::dotenv;

pub const DATABASE_URL: &str = dotenv!("DATABASE_URL");
pub const JWT_SECRET: &str = dotenv!("JWT_SECRET");
