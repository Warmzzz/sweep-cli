use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The content of one directory, read from the disk only once.
pub struct Directory {
    file_names: Vec<OsString>,
    subdirs: Vec<PathBuf>,
}

impl Directory {
    pub fn read(path: &Path) -> io::Result<Self> {
        let mut file_names = Vec::new();
        let mut subdirs = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            // `file_type` does not follow symbolic links,
            // so a linked folder is never walked into.
            if entry.file_type()?.is_dir() {
                subdirs.push(entry.path());
            } else {
                file_names.push(entry.file_name());
            }
        }

        Ok(Self {
            file_names,
            subdirs,
        })
    }

    pub fn has_file(&self, name: &str) -> bool {
        self.file_names.iter().any(|file_name| file_name == name)
    }

    pub fn has_file_with_extension(&self, extension: &str) -> bool {
        self.file_names
            .iter()
            .any(|file_name| Path::new(file_name).extension() == Some(OsStr::new(extension)))
    }

    /// Consumes the directory and hands back its sub-directories.
    pub fn into_subdirs(self) -> Vec<PathBuf> {
        self.subdirs
    }
}
