use std::path::PathBuf;

pub enum CopyError {
    NotFaster,
    SourceNotFound(PathBuf),
    CannotOverwrite(PathBuf),
    DirectoryCreationFailed(String),
    AccessDenied((PathBuf, PathBuf)),
    Other(String),
}

impl std::fmt::Debug for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFaster => {
                f.write_str("This isn't any faster for single files, just use cp/copy.")
            }
            Self::SourceNotFound(path) => {
                f.write_fmt(format_args!("Source path not found: {}", path.display()))
            }
            Self::CannotOverwrite(path) => f.write_fmt(format_args!(
                "Destination file already exists: {}",
                path.display()
            )),
            Self::DirectoryCreationFailed(error) => f.write_fmt(format_args!(
                "Could not create destination directory: {}",
                error
            )),
            Self::AccessDenied((src_path, dst_path)) => f.write_fmt(format_args!(
                "Access denied copying {} to {}",
                src_path.display(),
                dst_path.display()
            )),
            Self::Other(msg) => f.write_fmt(format_args!("Error: {}", msg)),
        }
    }
}
