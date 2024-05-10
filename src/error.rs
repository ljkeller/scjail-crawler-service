use std::fmt;

/// Simplified error wrappings for scjail_crawler_service.
#[derive(Debug)]
pub enum Error {
    /// Error related to network operations.
    NetworkError,
    /// Error related to parsing. Usually a client (read user) error.
    ParseError,
    /// Error related to invalid arguments. Usually a client (read user) error.
    ArgumentError,
    /// Error related to internal application logic, with an additional explanation.
    InternalError(String),
    /// Error related to PostgreSQL, with additional explanation
    PostgresError(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    /// Formats the error for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkError => write!(f, "Network error"),
            Error::ParseError => write!(f, "Parse error"),
            Error::ArgumentError => write!(f, "Argument error"),
            Error::InternalError(explanation) => write!(f, "Internal error: {}", explanation),
            Error::PostgresError(explanation) => {
                write!(f, "Internal Postgres error: {}", explanation)
            }
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::PostgresError(format!("Postgres error {}", e))
    }
}
