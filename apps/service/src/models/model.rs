#![allow(dead_code)]
use libsql::Connection;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Database query failed: {0}")]
    QueryFailure(#[from] libsql::Error),

    #[error("Record not found")]
    NotFound,

    #[error("The object is still ephemearl thus some things cannot be queried")]
    NotPesistent,

    #[error("Unique constraint violation")]
    Conflict,

    // #[error("Serialization error: {0}")]
    // Serialization(#[source] serde_json::Error),

    // #[error("Date parsing error: {0}")]
    // DateParse(#[from] chrono::ParseError),
    #[error("Custom error: {0}")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

/// Core trait for database models representing CRUD operations
///
/// Implement this trait for any struct that represents a database entity
/// to enable basic CRUD functionality.
pub trait Model: Sized {
    /// Error type for model operations
    type Error: std::error::Error + From<ModelError>;

    /// Retrieves a model instance by its primary key
    ///
    /// # Arguments
    /// * `id` - The primary key value to search for
    /// * `conn` - Database connection reference
    ///
    /// # Returns
    /// - `Ok(Some(Self))` if record found
    /// - `Ok(None)` if no record found
    /// - `Err(Self::Error)` on database failure
    ///
    /// # Example
    /// ```ignore
    /// let user = User::get_from_id(42, &conn).await?;
    /// ```
    async fn get_from_id(id: i64, conn: &Connection) -> Result<Option<Self>, Self::Error>;
    /// Persists the model instance to the database
    ///
    /// For new records (where ID is None), executes an INSERT and populates
    /// the ID field with the generated primary key.
    ///
    /// For existing records (where ID is Some), executes an UPDATE.
    ///
    /// # Arguments
    /// * `conn` - Database connection reference
    ///
    /// # Returns
    /// - `Ok(())` on successful persistence
    /// - `Err(Self::Error)` on database failure or constraint violation
    ///
    /// # Example
    /// ```ignore
    /// user.persist(&conn).await?;
    /// ```
    async fn persist(&mut self, conn: &Connection) -> Result<(), Self::Error>;

    /// Deletes the model instance from the database
    ///
    /// # Arguments
    /// * `conn` - Database connection reference
    ///
    /// # Returns
    /// - `Ok(())` on successful deletion
    /// - `Err(Self::Error)` if record doesn't exist or database failure
    async fn delete(self, conn: &Connection) -> Result<(), Self::Error>;
}
