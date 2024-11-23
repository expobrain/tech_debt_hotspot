mod formatting;
mod stats;
mod types;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{arg, command, value_parser};
use stats::{collect_changes_count_from_path, collect_stats_from_path, get_git_base_path};
use types::SortBy;

use crate::types::FileStats;

fn sort_stats_by(stats: &mut [FileStats], sort_by: SortBy) {
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
    }
}

fn main() {
    let matches = command!("tech_debt_hotspot")
        .arg(
            arg!(<DIRECTORY>)
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(-s --sort <SORT>).value_parser(value_parser!(SortBy)))
        .get_matches();

    let directory = matches
        .get_one::<PathBuf>("DIRECTORY")
        .unwrap()
        .canonicalize()
        .expect("Failed to canonicalize path");
    let sort_by = *matches
        .get_one::<SortBy>("sort")
        .unwrap_or(&SortBy::MaintainabilityIndex);

    if !directory.is_dir() || directory.read_dir().is_err() {
        eprintln!("Error: {} is not a directory", directory.display());
        std::process::exit(1);
    }

    let mut stats = HashMap::new();

    collect_stats_from_path(&directory, &mut stats);
    match collect_changes_count_from_path(&directory, &mut stats) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    let mut stats_vec: Vec<FileStats> = stats.values().cloned().collect();

    sort_stats_by(&mut stats_vec, sort_by);

    let git_base_path = get_git_base_path(&directory);

    for stat in &mut stats_vec {
        stat.path = PathBuf::from(&stat.path)
            .strip_prefix(&git_base_path)
            .unwrap()
            .to_string_lossy()
            .to_string()
            .to_string();
    }

    let table = formatting::format_markdown(&stats_vec);

    println!("{}", table);
}
