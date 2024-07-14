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
    /// Error related to AWS S3, with additional explanation
    S3Error(String),
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
            Error::S3Error(explanation) => write!(f, "S3 error: {}", explanation),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::PostgresError(format!("Postgres error {}", e))
    }
}

impl From<aws_sdk_s3::Error> for Error {
    fn from(e: aws_sdk_s3::Error) -> Self {
        Error::S3Error(format!("aws_sdk_3 error: {}", e))
    }
}

impl From<aws_sdk_s3::error::BuildError> for Error {
    fn from(e: aws_sdk_s3::error::BuildError) -> Self {
        Error::S3Error(format!("aws_smithy_types BuildError: {}", e))
    }
}

impl<T> From<aws_sdk_s3::error::SdkError<T>> for Error
where
    T: std::error::Error + Send + Sync + 'static,
{
    fn from(e: aws_sdk_s3::error::SdkError<T>) -> Self {
        Error::S3Error(format!("aws_sdk_3 SdkError: {}", e))
    }
}
