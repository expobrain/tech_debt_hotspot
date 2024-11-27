mod formatting;
mod hotspot;
mod sorting;
mod types;

use std::{
    error::Error,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use clap::{arg, command, value_parser};
use formatting::{Output, OutputFormat};
use hotspot::TechDebtHotspots;
use sorting::{sort_stats_by, SortBy};

fn to_canonicalised_path_buf(path: &Path) -> Result<PathBuf, String> {
    let canonicalised_path = path.canonicalize().unwrap();

    if !canonicalised_path.is_dir() || canonicalised_path.read_dir().is_err() {
        return Err(format!(
            "Error: {} is not a directory",
            canonicalised_path.display()
        ));
    }

    Ok(canonicalised_path)
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!("tech_debt_hotspot")
        .arg(
            arg!(<DIRECTORY>)
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(--sort <SORT>).value_parser(value_parser!(SortBy)))
        .arg(arg!(--exclude <EXCLUDE>).value_parser(value_parser!(PathBuf)))
        .arg(arg!(--since <SINCE>).value_parser(value_parser!(chrono::NaiveDate)))
        .arg(arg!(-o --output <OUTPUT>).value_parser(value_parser!(OutputFormat)))
        .get_matches();

    let directory = matches
        .get_one::<PathBuf>("DIRECTORY")
        .map(|path| to_canonicalised_path_buf(path))
        .unwrap()?;
    let exclude = matches
        .get_one::<PathBuf>("exclude")
        .map(|path| to_canonicalised_path_buf(path))
        .transpose()?;
    let since = matches.get_one::<NaiveDate>("since");
    let sort_by = *matches
        .get_one::<SortBy>("sort")
        .unwrap_or(&SortBy::MaintainabilityIndex);
    let output_format = *matches
        .get_one::<OutputFormat>("output")
        .unwrap_or(&OutputFormat::Markdown);

    let mut hotspot_stats = TechDebtHotspots::new();
    hotspot_stats.collect(&directory, exclude.as_deref(), since);

    let stats = sort_stats_by(hotspot_stats.stats(), sort_by);
    let output = Output::new(&output_format).format(&stats)?;

    println!("{}", output);

    Ok(())
}
