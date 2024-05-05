use std::fmt;

#[derive(Debug)]
pub enum Error {
    NetworkError,
    ParseError,
    ArgumentError,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkError => write!(f, "Network error"),
            Error::ParseError => write!(f, "Parse error"),
            Error::ArgumentError => write!(f, "Argument error"),
        }
    }
}
