//! Config admission validates and reads one securely opened local handle.

use std::{
    fs::{File, Metadata, OpenOptions},
    io::Read,
    path::Path,
};

pub(crate) fn read_bounded_regular_file(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    read_with_hook(path, limit, || {})
}

fn read_with_hook(path: &Path, limit: u64, after_open: impl FnOnce()) -> Result<Vec<u8>, String> {
    let file = open_secure(path)?;
    after_open();
    let metadata = file
        .metadata()
        .map_err(|_| "config handle metadata unavailable".to_string())?;
    validate_handle(&metadata, limit)?;
    let read_limit = limit
        .checked_add(1)
        .ok_or_else(|| "config size limit is invalid".to_string())?;
    let capacity = usize::try_from(read_limit.min(65_537))
        .map_err(|_| "config size limit is unsupported".to_string())?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| "config bounded read failed".to_string())?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err("config exceeds its byte limit".to_string());
    }
    Ok(bytes)
}

fn validate_handle(metadata: &Metadata, limit: u64) -> Result<(), String> {
    if !metadata.file_type().is_file() || handle_is_link_like(metadata) {
        return Err("config handle must be a regular non-link file".to_string());
    }
    if metadata.len() > limit {
        return Err("config exceeds its byte limit".to_string());
    }
    Ok(())
}

#[cfg(windows)]
fn handle_is_link_like(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn handle_is_link_like(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(unix)]
fn open_secure(path: &Path) -> Result<File, String> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "config could not be opened without following links".to_string())
}

#[cfg(windows)]
fn open_secure(path: &Path) -> Result<File, String> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| "config could not be opened without following links".to_string())
}

#[cfg(not(any(unix, windows)))]
fn open_secure(_path: &Path) -> Result<File, String> {
    Err("secure config admission is unsupported on this platform".to_string())
}

#[cfg(test)]
#[path = "file_input/tests.rs"]
mod tests;
