use crate::file_utils::make_relative_path;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::result::ZipResult;
use zip::write::FileOptions;
use zip::{write, CompressionMethod, ZipWriter};

/// Creates a zip archive that contains the files and directories from the specified directory.
pub fn zip_create_from_directory<P, Q>(archive_file: P, directory: Q) -> ZipResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let options = write::FileOptions::default().compression_method(CompressionMethod::Stored);
    zip_create_from_directory_with_options(archive_file, directory, options)
}

/// Creates a zip archive that contains the files and directories from the specified directory, uses the specified compression level.
pub fn zip_create_from_directory_with_options<P, Q>(
    archive_file: P,
    directory: Q,
    options: FileOptions,
) -> ZipResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let file = File::create(archive_file)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    zip_writer.create_from_directory_with_options(directory, options)
}

pub trait ZipWriterExtensions {
    /// Creates a zip archive that contains the files and directories from the specified directory.
    fn create_from_directory<P: AsRef<Path>>(&mut self, directory: P) -> ZipResult<()>;

    /// Creates a zip archive that contains the files and directories from the specified directory, uses the specified compression level.
    fn create_from_directory_with_options<P>(
        &mut self,
        directory: P,
        options: FileOptions,
    ) -> ZipResult<()>
    where
        P: AsRef<Path>;
}

impl<W: Write + io::Seek> ZipWriterExtensions for ZipWriter<W> {
    fn create_from_directory<P>(&mut self, directory: P) -> ZipResult<()>
    where
        P: AsRef<Path>,
    {
        let options = write::FileOptions::default().compression_method(CompressionMethod::Stored);
        self.create_from_directory_with_options(directory, options)
    }

    fn create_from_directory_with_options<P>(
        &mut self,
        directory: P,
        options: FileOptions,
    ) -> ZipResult<()>
    where
        P: AsRef<Path>,
    {
        let mut paths_queue: Vec<PathBuf> = vec![];
        paths_queue.push(directory.as_ref().into());

        let mut buffer = Vec::new();

        while let Some(next) = paths_queue.pop() {
            let directory_entry_iterator = std::fs::read_dir(next)?;

            for entry in directory_entry_iterator {
                let entry_path = entry?.path();
                let entry_metadata = std::fs::metadata(entry_path.clone())?;
                if entry_metadata.is_file() {
                    let mut f = File::open(&entry_path)?;
                    f.read_to_end(&mut buffer)?;
                    let relative_path = make_relative_path(&directory, &entry_path);
                    self.start_file_from_path(&relative_path, options)?;
                    self.write_all(buffer.as_ref())?;
                    buffer.clear();
                } else if entry_metadata.is_dir() {
                    let relative_path = make_relative_path(&directory, &entry_path);
                    self.add_directory_from_path(&relative_path, options)?;
                    paths_queue.push(entry_path);
                }
            }
        }

        self.finish()?;
        Ok(())
    }
}
