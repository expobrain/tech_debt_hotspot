use clap::{builder::PossibleValue, ValueEnum};

use crate::hotspot::HotstpoStats;

#[derive(Clone, Copy, Debug)]
pub enum SortBy {
    Path,
    MaintainabilityIndex,
    HalsteadVolume,
    CyclomaticComplexity,
    LinesOfCode,
    CommentsPercentage,
    ChangesCount,
    HotspotIndex,
}

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
            SortBy::HotspotIndex,
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
            SortBy::HotspotIndex => {
                PossibleValue::new("hotspot_index").help("Sort by hotspot index")
            }
        })
    }
}

pub fn sort_stats_by(mut stats: Vec<HotstpoStats>, sort_by: SortBy) -> Vec<HotstpoStats> {
    match sort_by {
        SortBy::Path => {
            stats.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        }
        SortBy::MaintainabilityIndex => {
            stats.sort_unstable_by(|a, b| {
                b.maitainability_index
                    .partial_cmp(&a.maitainability_index)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::HalsteadVolume => {
            stats.sort_unstable_by(|a, b| {
                b.halstead_volume
                    .partial_cmp(&a.halstead_volume)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::CyclomaticComplexity => {
            stats.sort_unstable_by(|a, b| {
                b.cyclomatic_complexity
                    .partial_cmp(&a.cyclomatic_complexity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::LinesOfCode => {
            stats.sort_unstable_by(|a, b| {
                b.loc
                    .partial_cmp(&a.loc)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::CommentsPercentage => {
            stats.sort_unstable_by(|a, b| {
                b.comments_percentage
                    .partial_cmp(&a.comments_percentage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::ChangesCount => {
            stats.sort_unstable_by(|a, b| {
                b.changes_count
                    .partial_cmp(&a.changes_count)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::HotspotIndex => {
            stats.sort_unstable_by(|a, b| {
                b.hotspot_index
                    .partial_cmp(&a.hotspot_index)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    };

    stats
}
