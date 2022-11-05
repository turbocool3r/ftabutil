use crate::error::FileOpError;
use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

fn read_file_impl(name: &'static str, path: &Path) -> Result<Vec<u8>, Box<FileOpError>> {
    let mut f = File::open(path)
        .map_err(|error| FileOpError::make_open(name, path.to_path_buf(), error))?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)
        .map_err(|error| FileOpError::make_read(name, path.to_path_buf(), error))?;
    Ok(v)
}

/// Reads a file from the specified path.
///
/// # Errors
/// This function will return a boxed `FileOpError` with either the `FileOpAction::Open` or the
/// `FileOpAction::Read` action in case an I/O error occurs while opening or reading the file.
pub fn read_file<P: AsRef<Path>>(name: &'static str, path: P) -> Result<Vec<u8>, Box<FileOpError>> {
    read_file_impl(name, path.as_ref())
}

fn create_file_impl(
    name: &'static str,
    path: &Path,
    overwrite: bool,
) -> Result<File, Box<FileOpError>> {
    OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(path)
        .map_err(|error| FileOpError::make_create(name, path.to_path_buf(), error))
}

/// Creates a file at the specified path.
///
/// In case the `overwrite` argument is `true`, the file will be either created or truncated if it
/// exists, otherwise an existing file at the specified path will cause an error to be returned.
///
/// # Errors
/// This function will return a boxed `FileOpError` with the `FileOpAction::Create` action in case
/// an I/O error occurs while creating the file.
pub fn create_file<P: AsRef<Path>>(
    name: &'static str,
    path: P,
    overwrite: bool,
) -> Result<File, Box<FileOpError>> {
    create_file_impl(name, path.as_ref(), overwrite)
}

fn save_file_impl(
    name: &'static str,
    path: &Path,
    data: &[u8],
    overwrite: bool,
) -> Result<(), Box<FileOpError>> {
    create_file(name, path, overwrite)?
        .write_all(data)
        .map_err(|error| FileOpError::make_write(name, path.to_path_buf(), error))?;

    info!("Saved {} to {}.", name, path.display());

    Ok(())
}

/// Creates a file at the specified path and writes data from a slice into it.
///
/// In case the `overwrite` argument is `true`, the file will be either created or truncated and
/// overwritten if it exists, otherwise an existing file at the specified path will cause an error
/// to be returned.
///
/// File creation is handled by the [`create_file`] function internally.
///
/// # Errors
/// This function will return a boxed [`FileOpError`] with either [`FileOpAction::Create`] or
/// [`FileOpAction::Write`] action in case an I/O error occurs while either creating or writing the
/// file.
pub fn save_file<P: AsRef<Path>>(
    name: &'static str,
    path: P,
    data: &[u8],
    overwrite: bool,
) -> Result<(), Box<FileOpError>> {
    save_file_impl(name, path.as_ref(), data, overwrite)
}

fn qualify_path_if_needed_impl<'a>(path: &'a Path, dir: Option<&Path>) -> Cow<'a, Path> {
    if path.is_absolute() {
        Cow::from(path)
    } else if let Some(dir) = dir {
        let mut new_path = dir.to_path_buf();
        new_path.push(path);
        Cow::from(new_path)
    } else {
        Cow::from(path)
    }
}

/// Qualifies a path with another path if needed:
///
/// * In case the path is relative, it is qualified and a new `PathBuf` with the qualified path is
///   returned,
/// * In case the path is absolute, it is returned as is.
pub fn qualify_path_if_needed<'a, P, D>(path: &'a P, dir: Option<&D>) -> Cow<'a, Path>
where
    P: AsRef<Path> + ?Sized,
    D: AsRef<Path> + ?Sized,
{
    qualify_path_if_needed_impl(path.as_ref(), dir.map(AsRef::as_ref))
}

/// Takes either a path or a default path in case the `path` argument is `None` and qualifies it
/// with a directory path if needed:
///
/// * In case the path is relative, it is qualified and a new `PathBuf` with the qualified path is
///   returned,
/// * In case the path is absolute, it is returned as is.
///
/// This function is a convenience wrapper around the [`qualify_path_if_needed`] function.
pub fn qualify_path_or_default_if_needed<'a, P, D, Q>(
    path: Option<&'a P>,
    dir: Option<&D>,
    default: &'a Q,
) -> Cow<'a, Path>
where
    P: AsRef<Path> + ?Sized,
    D: AsRef<Path> + ?Sized,
    Q: AsRef<Path> + ?Sized,
{
    let path = path.map(AsRef::as_ref).unwrap_or_else(|| default.as_ref());
    qualify_path_if_needed(path, dir)
}
