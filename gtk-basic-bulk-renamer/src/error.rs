use regex::Error as RegexError;
use std::error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum Error {
    Regex(RegexError),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Regex(e) => e.source(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Regex(e) => e.fmt(f),
        }
    }
}

impl From<RegexError> for Error {
    fn from(e: RegexError) -> Self {
        Self::Regex(e)
    }
}
