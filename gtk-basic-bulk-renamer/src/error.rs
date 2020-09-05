use basic_bulk_renamer::RenameError;
use regex::Error as RegexError;
use std::error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum Error {
    Rename(RenameError),
    Regex(RegexError),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Rename(e) => Some(e),
            Error::Regex(e) => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Rename(e) => e.fmt(f),
            Error::Regex(e) => e.fmt(f),
        }
    }
}

impl From<RenameError> for Error {
    fn from(e: RenameError) -> Self {
        Self::Rename(e)
    }
}

impl From<RegexError> for Error {
    fn from(e: RegexError) -> Self {
        Self::Regex(e)
    }
}
