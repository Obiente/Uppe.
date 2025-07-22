use std::sync::atomic::AtomicUsize;

use deadpool::managed::{self, Pool, RecycleResult};
use libsql::{Connection, Database, Error as LibsqlError, params};

pub struct LibsqlManager {
    database: Database,
    recycle_count: AtomicUsize,
}

impl LibsqlManager {
    pub fn new(database: Database) -> Self {
        Self { database, recycle_count: AtomicUsize::new(0) }
    }
}

impl managed::Manager for LibsqlManager {
    type Type = Connection;
    type Error = LibsqlError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        self.database.connect()
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _: &managed::Metrics,
    ) -> RecycleResult<Self::Error> {
        let recycle_count = self.recycle_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let row = conn
            .query("SELECT ?1", params![recycle_count as u64])
            .await?
            .next()
            .await?
            .ok_or(LibsqlError::QueryReturnedNoRows)?;
        assert!(recycle_count as u64 == row.get::<u64>(0)?);
        Ok(())
    }
}

pub type LibsqlPool = Pool<LibsqlManager>;
