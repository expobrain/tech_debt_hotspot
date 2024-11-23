use std::fmt;

use clap::{builder::PossibleValue, ValueEnum};
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

#[derive(Clone, Copy, Debug)]
pub enum SortBy {
    Path,
    MaintainabilityIndex,
    HalsteadVolume,
    CyclomaticComplexity,
    LinesOfCode,
    CommentsPercentage,
    ChangesCount,
}

// Can also be derived with feature flag `derive`
impl ValueEnum for SortBy {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            SortBy::Path,
            SortBy::MaintainabilityIndex,
            SortBy::HalsteadVolume,
            SortBy::CyclomaticComplexity,
            SortBy::LinesOfCode,
            SortBy::CommentsPercentage,
            SortBy::ChangesCount,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            SortBy::Path => PossibleValue::new("path").help("Sort by path"),
            SortBy::MaintainabilityIndex => {
                PossibleValue::new("maintainability_index").help("Sort by maintainability index")
            }
            SortBy::HalsteadVolume => {
                PossibleValue::new("halstead_volume").help("Sort by Halstead volume")
            }
            SortBy::CyclomaticComplexity => {
                PossibleValue::new("cyclomatic_complexity").help("Sort by cyclomatic complexity")
            }
            SortBy::LinesOfCode => {
                PossibleValue::new("lines_of_code").help("Sort by lines of code")
            }
            SortBy::CommentsPercentage => {
                PossibleValue::new("comments_percentage").help("Sort by comments percentage")
            }
            SortBy::ChangesCount => {
                PossibleValue::new("changes_count").help("Sort by changes count")
            }
        })
    }
}
