use crate::parser::{OobSegmentError, ParseError};
use std::{
    error::Error,
    fmt, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// File actions that are supported by the [`FileOpError`] type.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum FileOpAction {
    /// Specifies that an error occurred while trying to create a file.
    Create,
    /// Specifies that an error occurred while opening an existing file.
    Open,
    /// Specifies that an error occurred while reading from a file.
    Read,
    /// Specifies that an error occurred while writing to a file.
    Write,
}

/// An error type that contains enough information to display an error which occurred during a file
/// I/O operation.
#[derive(Debug)]
pub struct FileOpError {
    /// The action which caused an error.
    pub action: FileOpAction,
    /// The name of the file to be included into the error message.
    pub name: &'static str,
    /// The path to the file on which the I/O operation was performed.
    pub path: PathBuf,
    /// The error returned by the I/O operation.
    pub error: io::Error,
}

impl FileOpError {
    /// Creates a boxed [`FileOpError`].
    pub fn boxed(
        action: FileOpAction,
        name: &'static str,
        path: PathBuf,
        error: io::Error,
    ) -> Box<Self> {
        Box::new(Self {
            action,
            name,
            path,
            error,
        })
    }

    /// Creates a boxed [`FileOpError`] setting action to [`FileOpAction::Create`].
    pub fn make_create(name: &'static str, path: PathBuf, error: io::Error) -> Box<Self> {
        Self::boxed(FileOpAction::Create, name, path, error)
    }

    /// Creates a boxed [`FileOpError`] setting action to [`FileOpAction::Open`].
    pub fn make_open(name: &'static str, path: PathBuf, error: io::Error) -> Box<Self> {
        Self::boxed(FileOpAction::Open, name, path, error)
    }

    /// Creates a boxed [`FileOpError`] setting action to [`FileOpAction::Read`].
    pub fn make_read(name: &'static str, path: PathBuf, error: io::Error) -> Box<Self> {
        Self::boxed(FileOpAction::Read, name, path, error)
    }

    /// Creates a boxed [`FileOpError`] setting action to [`FileOpAction::Write`].
    pub fn make_write(name: &'static str, path: PathBuf, error: io::Error) -> Box<Self> {
        Self::boxed(FileOpAction::Write, name, path, error)
    }
}

impl fmt::Display for FileOpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let verb = match self.action {
            FileOpAction::Create => "create",
            FileOpAction::Open => "open",
            FileOpAction::Read => "read",
            FileOpAction::Write => "write",
        };

        write!(
            f,
            "failed to {} {} at path {}: {}",
            verb,
            self.name,
            self.path.display(),
            self.error
        )
    }
}

impl Error for FileOpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// A type that describes errors which may be returned by the `pack` operation.
#[derive(Debug, Error)]
pub enum PackError<'a> {
    /// A catch-all for all file I/O errors.
    #[error("{0}")]
    FileOp(#[from] Box<FileOpError>),
    /// An error that may occur during manifest parsing.
    #[error("failed to parse the manifest file at {}: {}", .0.display(), .1)]
    ManifestParseError(&'a Path, #[source] toml::de::Error),
}

/// A type that describes errors which may be returned by the `unpack` operation.
#[derive(Debug, Error)]
pub enum UnpackError<'a> {
    /// A catch-all for all file I/O errors.
    #[error("{0}")]
    FileOp(#[from] Box<FileOpError>),
    /// An error returned when the output directory path points to something other than a directory.
    #[error("path {} exists and is not a directory.", .0.display())]
    OutDirIsNotDir(&'a Path),
    /// An error returned when the output directory couldn't be created.
    #[error("couldn't create target directory at {}: {}", .0.display(), .1)]
    FailedToCreateOutDir(&'a Path, #[source] io::Error),
    /// An error returned when the 'ftab' file parser fails while parsing the header.
    #[error("failed to parse file at {}: {}", .0.display(), .1)]
    HeaderParseError(&'a Path, #[source] ParseError),
    /// An error returned when a segment header of a 'ftab' file specifies an out of bounds range.
    #[error("{0}")]
    OobSegmentError(#[from] OobSegmentError),
}
