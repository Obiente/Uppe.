#![allow(dead_code)]
use crate::models::model::{Model, ModelError};
use uuid::Uuid;
#[derive(Debug)]
pub struct Monitor {
    id: i64,
    uuid: Uuid,
}
impl Model for Monitor {
    type Error = ModelError;

    async fn get_from_id(id: i64, _conn: &libsql::Connection) -> Result<Option<Self>, Self::Error> {
        Ok(Some(Monitor { id, uuid: Uuid::new_v4() })) // Placeholder implementation
    }
    async fn persist(&mut self, _conn: &libsql::Connection) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn delete(self, _conn: &libsql::Connection) -> Result<(), Self::Error> {
        Ok(())
    }
}
