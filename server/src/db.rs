use std::sync::Arc;

#[derive(Debug)]
pub struct Auth {
    pub id: i64,
    pub username: Arc<str>,
    pub password: Arc<str>,
}

#[derive(Debug)]
pub struct Player {
    pub id: i64,
    pub auth_id: i64,
    pub nickname: Arc<str>,
    pub color: i64,
    pub best_score: i64,
}
