use std::io::Error as IoError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0:#}")]
    Io(#[from] IoError),
    #[error("Address parsing error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
}
