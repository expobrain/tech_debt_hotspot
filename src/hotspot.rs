use chrono::NaiveDate;
use core::panic;
use rust_code_analysis::ParserTrait;
use rust_code_analysis::{metrics, PythonParser};
use std::path::PathBuf;
use std::{collections::HashMap, fs, path::Path, process::Command};
use tabled::Tabled;

use crate::types::PathType;

#[derive(Clone, Default)]
struct FileStats {
    pub path: PathBuf,
    pub path_type: PathType,
    pub halstead_volume: f64,
    pub cyclomatic_complexity: f64,
    pub loc: u32,
    pub comments_percentage: f64,
    pub maintainability_index: f64,
    pub changes_count: u32,
}

#[derive(Tabled)]
pub struct HotstpoStats {
    pub path: String,
    pub path_type: PathType,
    pub halstead_volume: f64,
    pub cyclomatic_complexity: f64,
    pub loc: u32,
    pub comments_percentage: f64,
    pub maintainability_index: f64,
    pub changes_count: u32,
    pub hotspot_index: f64,
}

impl HotstpoStats {
    fn new(file_stats: &FileStats) -> HotstpoStats {
        HotstpoStats {
            path: file_stats.path.display().to_string(),
            path_type: file_stats.path_type.clone(),
            halstead_volume: file_stats.halstead_volume,
            cyclomatic_complexity: file_stats.cyclomatic_complexity,
            loc: file_stats.loc,
            comments_percentage: file_stats.comments_percentage,
            maintainability_index: file_stats.maintainability_index,
            changes_count: file_stats.changes_count,
            hotspot_index: file_stats.changes_count as f64
                / (file_stats.maintainability_index / 100.0),
        }
    }
}

#[derive(Default)]
pub struct TechDebtHotspots {
    git_base_path: PathBuf,
    path: PathBuf,
    exclude: Option<PathBuf>,
    since: Option<NaiveDate>,
    stats: HashMap<PathBuf, FileStats>,
}

impl TechDebtHotspots {
    pub fn new() -> Self {
        TechDebtHotspots::default()
    }

    pub fn stats(&self) -> Vec<HotstpoStats> {
        self.stats.values().map(HotstpoStats::new).collect()
    }

    pub fn collect(&mut self, directory: &Path, exclude: Option<&Path>, since: Option<&NaiveDate>) {
        self.path = directory.to_path_buf();
        self.exclude = exclude.map(|p| p.to_path_buf());
        self.since = since.cloned();
        self.git_base_path = Self::get_git_base_path(directory);

        self.collect_filenames()
            .get_stats_from_filenames()
            .collect_changes_count()
            .compute_directory_stats()
            .normalise_to_git_root();
    }

    fn compute_directory_stats(&mut self) -> &mut Self {
        for (path, file_stats) in self.stats.clone().iter() {
            let mut paths_to_visit = vec![path.parent().unwrap()];

            match paths_to_visit.pop() {
                None => {}
                Some(path) if path == self.git_base_path => {}
                Some(path) => {
                    paths_to_visit.push(path.parent().unwrap());

                    let directory_stats =
                        self.stats
                            .entry(path.to_path_buf())
                            .or_insert_with(|| FileStats {
                                path: path.to_path_buf(),
                                path_type: PathType::Directory,
                                ..Default::default()
                            });

                    directory_stats.halstead_volume = match file_stats.halstead_volume.is_nan() {
                        true => directory_stats.halstead_volume,
                        false => directory_stats.halstead_volume + file_stats.halstead_volume,
                    };
                    directory_stats.cyclomatic_complexity =
                        match file_stats.cyclomatic_complexity.is_nan() {
                            true => directory_stats.cyclomatic_complexity,
                            false => {
                                directory_stats.cyclomatic_complexity
                                    + file_stats.cyclomatic_complexity
                            }
                        };
                    directory_stats.loc += file_stats.loc;
                    directory_stats.comments_percentage = match directory_stats.loc + file_stats.loc
                    {
                        0 => 0.0,
                        _ => {
                            (directory_stats.comments_percentage * directory_stats.loc as f64
                                + file_stats.comments_percentage * file_stats.loc as f64)
                                / (directory_stats.loc + file_stats.loc) as f64
                        }
                    };
                    directory_stats.maintainability_index = f64::min(
                        directory_stats.maintainability_index,
                        file_stats.maintainability_index,
                    );
                    directory_stats.changes_count += file_stats.changes_count;
                }
            }
        }

        self
    }

    fn collect_filenames(&mut self) -> &mut Self {
        let mut paths_to_visit = vec![self.path.clone()];

        while let Some(current_path) = paths_to_visit.pop() {
            if let Some(ref exclude) = self.exclude {
                if current_path.starts_with(exclude) {
                    continue;
                }
            }

            match current_path.is_dir() {
                true => {
                    current_path.read_dir().unwrap().for_each(|entry| {
                        let path_to_visit = entry.unwrap().path();
                        paths_to_visit.push(path_to_visit);
                    });
                }
                false if current_path.extension().and_then(|s| s.to_str()) == Some("py") => {
                    self.stats.insert(
                        current_path.to_path_buf(),
                        FileStats {
                            path: current_path,
                            path_type: PathType::File,
                            ..Default::default()
                        },
                    );
                }
                _ => {}
            }
        }

        self
    }

    pub fn collect_changes_count(&mut self) -> &mut Self {
        let mut command = Command::new("git");

        command
            .current_dir(self.path.clone())
            .arg("log")
            .arg("--name-only")
            .arg("--pretty=format:");

        if let Some(since) = self.since {
            command.arg(format!("--since={}", since));
        }

        let output = command
            .arg(".")
            .output()
            .map_err(|e| format!("Failed to execute git command: {}", e))
            .unwrap();

        if !output.status.success() {
            panic!(
                "Git command failed with status {}: {:?}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse git output: {}", e))
            .unwrap();
        let lines = stdout.lines().filter(|line| !line.trim().is_empty());

        for line in lines {
            let filename_path = PathBuf::from(line);
            let absolute_path = self.git_base_path.join(&filename_path);

            if !absolute_path.exists() {
                continue;
            }

            // update filename stats
            if let Some(existing) = self.stats.get_mut(&absolute_path) {
                existing.changes_count += 1;
            };
        }

        self
    }

    fn get_stats_from_filenames(&mut self) -> &mut Self {
        for (_, file_stats) in self.stats.iter_mut() {
            Self::get_stats_from_filename(file_stats);
        }

        self
    }

    fn get_stats_from_filename(file_stats: &mut FileStats) {
        let path = Path::new(&file_stats.path).to_path_buf();
        let source_code = fs::read(path.clone()).unwrap();
        let parser = PythonParser::new(source_code, &path, None);

        if let Some(s) = metrics(&parser, &path) {
            file_stats.path = path;
            file_stats.path_type = PathType::File;
            file_stats.halstead_volume = s.metrics.halstead.volume();
            file_stats.cyclomatic_complexity = s.metrics.cyclomatic.cyclomatic_max();
            file_stats.loc = s.metrics.loc.sloc() as u32;
            file_stats.comments_percentage = s.metrics.loc.cloc() / s.metrics.loc.sloc() * 100.0;
            file_stats.maintainability_index = s.metrics.mi.mi_visual_studio();
        };
    }

    fn normalise_to_git_root(&mut self) -> &mut Self {
        for (_, file_stats) in self.stats.iter_mut() {
            let path = Path::new(&file_stats.path).to_path_buf();
            let relative_path = path.strip_prefix(&self.git_base_path).unwrap();
            file_stats.path = relative_path.to_path_buf();
        }

        self
    }

    fn get_git_base_path(directory: &Path) -> PathBuf {
        let output = Command::new("git")
            .current_dir(directory)
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output()
            .unwrap();

        if !output.status.success() {
            panic!(
                "Failed to get git base path: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout).unwrap();
        let path = PathBuf::from(stdout.trim());

        path
    }
}
