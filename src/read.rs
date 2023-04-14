use crate::file_utils::file_write_all_bytes;
use std::fs::File;
use std::io;
use std::io::{Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use zip::read::ZipFile;
use zip::result::{ZipError, ZipResult};
use zip::ZipArchive;

/// Extracts a ZIP file to the given directory.
pub fn zip_extract<P, Q>(archive_file: P, target_dir: Q) -> ZipResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let file = File::open(archive_file)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(target_dir)
}

/// Extracts and entry in the ZIP archive to the given directory.
pub fn zip_extract_file<P, Q, R>(
    archive_file: P,
    entry_path: Q,
    target_dir: R,
    overwrite: bool,
) -> ZipResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let file = File::open(archive_file)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let file_number: usize = match archive.file_number(entry_path.as_ref()) {
        Some(index) => index,
        None => return Err(ZipError::FileNotFound),
    };
    let destination_file_path = target_dir.as_ref().join(entry_path);
    archive.extract_file(file_number, &destination_file_path, overwrite)
}

/// Extracts an entry in the ZIP archive to the given memory buffer.
pub fn zip_extract_file_to_memory<P, Q>(
    archive_file: P,
    entry_path: Q,
    buffer: &mut Vec<u8>,
) -> ZipResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let file = File::open(archive_file)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let file_number: usize = match archive.file_number(entry_path.as_ref()) {
        Some(index) => index,
        None => return Err(ZipError::FileNotFound),
    };
    archive.extract_file_to_memory(file_number, buffer)
}

/// Determines whether the specified file is a ZIP file, or not.
pub fn try_is_zip<P>(file: P) -> ZipResult<bool>
where
    P: AsRef<Path>,
{
    const ZIP_SIGNATURE: [u8; 2] = [0x50, 0x4b];
    const ZIP_ARCHIVE_FORMAT: [u8; 6] = [0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let mut file = File::open(file)?;
    let mut buffer: [u8; 4] = [0; 4];
    let bytes_read = file.read(&mut buffer)?;
    if bytes_read == buffer.len() {
        for i in 0..ZIP_SIGNATURE.len() {
            if buffer[i] != ZIP_SIGNATURE[i] {
                return Ok(false);
            }
        }

        for i in (0..ZIP_ARCHIVE_FORMAT.len()).step_by(2) {
            if buffer[2] == ZIP_ARCHIVE_FORMAT[i] || buffer[3] == ZIP_ARCHIVE_FORMAT[i + 1] {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Determines whether the specified file is a ZIP file, or not.
pub fn is_zip<P: AsRef<Path>>(file: P) -> bool {
    try_is_zip(file).unwrap_or_default()
}

pub trait ZipArchiveExtensions {
    /// Extracts the current archive to the given directory path.
    fn extract<P: AsRef<Path>>(&mut self, path: P) -> ZipResult<()>;

    /// Extracts an entry in the zip archive to a file.
    fn extract_file<P>(
        &mut self,
        file_number: usize,
        destination_file_path: P,
        overwrite: bool,
    ) -> ZipResult<()>
    where
        P: AsRef<Path>;

    /// Extracts an entry in the ZIP archive to the given memory buffer.
    fn extract_file_to_memory(&mut self, file_number: usize, buffer: &mut Vec<u8>)
        -> ZipResult<()>;

    /// Gets an entry´s path.
    fn entry_path(&mut self, file_number: usize) -> ZipResult<PathBuf>;

    /// Finds the index of the specified entry.
    fn file_number<P: AsRef<Path>>(&mut self, entry_path: P) -> Option<usize>;
}

impl<R: Read + io::Seek> ZipArchiveExtensions for ZipArchive<R> {
    fn extract<P: AsRef<Path>>(&mut self, target_directory: P) -> ZipResult<()> {
        if !target_directory.as_ref().is_dir() {
            return Err(ZipError::Io(Error::new(
                ErrorKind::InvalidInput,
                "The specified path does not indicate a valid directory path.",
            )));
        }

        for file_number in 0..self.len() {
            let mut next: ZipFile = self.by_index(file_number)?;
            let sanitized_name = next.sanitized_name();
            if next.is_dir() {
                let extracted_folder_path = target_directory.as_ref().join(sanitized_name);
                std::fs::create_dir_all(extracted_folder_path)?;
            } else if next.is_file() {
                let mut buffer: Vec<u8> = Vec::new();
                let _bytes_read = next.read_to_end(&mut buffer)?;
                let extracted_file_path = target_directory.as_ref().join(sanitized_name);
                file_write_all_bytes(extracted_file_path, buffer.as_ref(), true)?;
            }
        }

        Ok(())
    }

    fn extract_file<P>(
        &mut self,
        file_number: usize,
        destination_file_path: P,
        overwrite: bool,
    ) -> ZipResult<()>
    where
        P: AsRef<Path>,
    {
        let mut buffer: Vec<u8> = Vec::new();
        self.extract_file_to_memory(file_number, &mut buffer)?;
        file_write_all_bytes(destination_file_path, buffer.as_ref(), overwrite)?;
        Ok(())
    }

    fn extract_file_to_memory(
        &mut self,
        file_number: usize,
        buffer: &mut Vec<u8>,
    ) -> ZipResult<()> {
        let mut next: ZipFile = self.by_index(file_number)?;
        if next.is_file() {
            let _bytes_read = next.read_to_end(buffer)?;
            return Ok(());
        }
        Err(ZipError::Io(Error::new(
            ErrorKind::InvalidInput,
            "The specified index does not indicate a file entry.",
        )))
    }

    fn entry_path(&mut self, file_number: usize) -> ZipResult<PathBuf> {
        let next: ZipFile = self.by_index(file_number)?;
        Ok(next.sanitized_name())
    }

    fn file_number<P>(&mut self, entry_path: P) -> Option<usize>
    where
        P: AsRef<Path>,
    {
        for file_number in 0..self.len() {
            if let Ok(next) = self.by_index(file_number) {
                let sanitized_name = next.sanitized_name();
                if sanitized_name.as_path() == entry_path.as_ref() {
                    return Some(file_number);
                }
            }
        }
        None
    }
}
