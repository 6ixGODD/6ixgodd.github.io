use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid workspace: {0}")]
    InvalidWorkspace(String),

    #[error("invalid slug: {0}")]
    InvalidSlug(String),

    #[error("{path}:{line}: {message}")]
    Source {
        path: PathBuf,
        line: usize,
        message: String,
    },

    #[error("server error: {0}")]
    Server(String),
}

pub trait IoContext<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn at(self, path: impl Into<PathBuf>) -> Result<T> {
        let path = path.into();
        self.map_err(|source| Error::Io { path, source })
    }
}
