use crate::models::model::{Model, ModelError};
use uuid::Uuid;
#[derive(Debug)]
pub struct Monitor {
    id: i64,
    uuid: Uuid,
}
impl Monitor {
    type Error = ModelError;
    async fn get_from_uuid(
        id: Uuid,
        conn: &libsql::Connection,
    ) -> Result<Option<Self>, Self::Error> {
        let params = [id.to_string()];
        conn.query("SELECT * FROM monitors WHERE id = ?", params)
            .await
            .expect("Failed to get monitor by UUID")
            .next()
            .await?
            .map(|row| {
                Some(Monitor {
                    id: row.get::<i64>(0).expect("Failed to get id from row"),
                    uuid: Uuid::parse_str(
                        &row.get::<String>(1).expect("Failed to get uuid from row"),
                    )
                    .unwrap(),
                })
            })
            .ok_or(ModelError::NotFound)
    }
}
impl Model for Monitor {
    type Error = ModelError;

    async fn get_from_id(id: i64, conn: &libsql::Connection) -> Result<Option<Self>, Self::Error> {
        Ok(Some(Monitor { id, uuid: Uuid::new_v4() })) // Placeholder implementation
    }
    async fn persist(&mut self, conn: &libsql::Connection) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn delete(self, conn: &libsql::Connection) -> Result<(), Self::Error> {
        Ok(())
    }
}
