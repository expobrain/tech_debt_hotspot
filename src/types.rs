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

#[derive(Debug, Tabled, Clone, Default)]
pub struct FileStats {
    pub path: String,
    pub path_type: PathType,
    pub halstead_volume: f64,
    pub cyclomatic_complexity: f64,
    pub loc: f64,
    pub comments_percentage: f64,
    pub maitainability_index: f64,
    pub changes_count: u32,
}
