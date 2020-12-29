use crate::basic_bulk_renamer::RenameError;
use regex::Error as RegexError;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error(transparent)]
    Rename(#[from] RenameError),
    #[error(transparent)]
    Regex(#[from] RegexError),
}
