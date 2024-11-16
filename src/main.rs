mod formatting;
mod stats;
mod types;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{arg, command, value_parser};
use stats::{collect_changes_count_from_path, collect_stats_from_path, get_git_base_path};

use crate::types::FileStats;

fn main() {
    let matches = command!("tech_debt_hotspot")
        .arg(
            arg!([DIRECTORY])
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .get_matches();

    let directory = matches
        .get_one::<PathBuf>("DIRECTORY")
        .unwrap()
        .canonicalize()
        .expect("Failed to canonicalize path");

    if !directory.is_dir() || directory.read_dir().is_err() {
        eprintln!("Error: {} is not a directory", directory.display());
        std::process::exit(1);
    }

    println!("Using input file: {}", directory.display());

    let mut stats = HashMap::new();

    collect_stats_from_path(&directory, &mut stats);
    match collect_changes_count_from_path(&directory, &mut stats) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    let git_base_path = get_git_base_path(&directory);
    let mut stats_vec: Vec<FileStats> = stats.values().cloned().collect();
    stats_vec.sort_unstable_by(|a, b| {
        b.maitainability_index
            .partial_cmp(&a.maitainability_index)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

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
