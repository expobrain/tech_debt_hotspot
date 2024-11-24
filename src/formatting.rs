use tabled::settings::object::Segment;
use tabled::settings::{Alignment, Modify, Style};
use tabled::Table;

use crate::hotspot::FileStats;

pub fn format_markdown(stats: &[&FileStats]) -> String {
    Table::new(stats)
        .with(Style::markdown())
        .with(Modify::new(Segment::new(1.., 2..)).with(Alignment::right()))
        .to_string()
}
