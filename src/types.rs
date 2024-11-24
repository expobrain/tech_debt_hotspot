use std::fmt;

use tabled::Tabled;

#[derive(Debug, Tabled, Clone, Default)]
pub enum PathType {
    #[default]
    File,
    Directory,
}

impl fmt::Display for PathType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PathType::Directory => write!(f, "directory"),
            PathType::File => write!(f, "file"),
        }
    }
}
