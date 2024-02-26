use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    db: PgPool,
}

impl AppState {
    pub fn new(db: String) -> sqlx::Result<Self> {
        let db = PgPool::connect_lazy(&db)?;

        Ok(Self { db })
    }

    pub fn connection(&self) -> &PgPool {
        &self.db
    }
}
