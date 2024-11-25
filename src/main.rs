mod formatting;
mod hotspot;
mod sorting;
mod types;

use std::path::PathBuf;

use clap::{arg, command, value_parser};
use hotspot::TechDebtHotspots;
use sorting::{sort_stats_by, SortBy};

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

    let mut hotspot_stats = TechDebtHotspots::new();
    hotspot_stats.collect(&directory);

    let stats = sort_stats_by(hotspot_stats.stats(), sort_by);
    let table = formatting::format_markdown(&stats);

    println!("{}", table);
}
