use std::error::Error;

use clap::builder::PossibleValue;
use clap::ValueEnum;
use csv::Writer;
use tabled::settings::object::Segment;
use tabled::settings::{Alignment, Modify, Style};
use tabled::Table;

use crate::hotspot::HotspotStats;

#[derive(Clone, Copy)]
pub enum OutputFormat {
    Markdown,
    Csv,
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[OutputFormat::Markdown, OutputFormat::Csv]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            OutputFormat::Markdown => PossibleValue::new("markdown").help("Markdown format"),
            OutputFormat::Csv => PossibleValue::new("csv").help("CSV format"),
        })
    }
}

pub struct Output {
    format: OutputFormat,
}

impl Output {
    pub fn new(format: &OutputFormat) -> Self {
        Output { format: *format }
    }

    pub fn format(&self, stats: &[HotspotStats]) -> Result<String, Box<dyn Error>> {
        match self.format {
            OutputFormat::Markdown => Self::format_markdown(stats),
            OutputFormat::Csv => Self::format_csv(stats),
        }
    }

    fn format_csv(stats: &[HotspotStats]) -> Result<String, Box<dyn Error>> {
        let mut writer = Writer::from_writer(vec![]);

        writer.write_record([
            "path",
            "path_type",
            "halstead_volume",
            "cyclomatic_complexity",
            "loc",
            "comments_percentage",
            "maintainability_index",
            "changes_count",
            "hotspot_index",
        ])?;

        for stat in stats {
            writer
                .serialize(&[
                    stat.path.clone(),
                    stat.halstead_volume.to_string(),
                    stat.cyclomatic_complexity.to_string(),
                    stat.loc.to_string(),
                    stat.comments_percentage.to_string(),
                    stat.maintainability_index.to_string(),
                    stat.changes_count.to_string(),
                    stat.hotspot_index.to_string(),
                ])
                .unwrap();
        }

        writer.flush()?;

        let output = String::from_utf8(writer.into_inner()?)?;

        Ok(output)
    }

    fn format_markdown(stats: &[HotspotStats]) -> Result<String, Box<dyn Error>> {
        Ok(Table::new(stats)
            .with(Style::markdown())
            .with(Modify::new(Segment::new(1.., 2..)).with(Alignment::right()))
            .to_string())
    }
}
