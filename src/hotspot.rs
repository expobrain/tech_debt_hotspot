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
    pub maitainability_index: f64,
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
    pub maitainability_index: f64,
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
            maitainability_index: file_stats.maitainability_index,
            changes_count: file_stats.changes_count,
            hotspot_index: file_stats.changes_count as f64 / file_stats.maitainability_index,
        }
    }
}

#[derive(Default)]
pub struct TechDebtHotspots {
    stats: HashMap<PathBuf, FileStats>,
}

impl TechDebtHotspots {
    pub fn new() -> Self {
        TechDebtHotspots::default()
    }
    pub fn stats(&self) -> Vec<HotstpoStats> {
        self.stats.values().map(HotstpoStats::new).collect()
    }

    pub fn collect(&mut self, path: &Path) {
        self.collect_filenames(path)
            .get_stats_from_filenames()
            .collect_changes_count(path)
            .compute_directory_stats(path)
            .normalise_to_git_root(path);
    }

    fn compute_directory_stats(&mut self, path: &Path) -> &mut Self {
        let root_path = Self::get_git_base_path(path);

        for (path, file_stats) in self.stats.clone().iter() {
            let mut paths_to_visit = vec![path.parent().unwrap()];

            match paths_to_visit.pop() {
                None => {}
                Some(path) if path == root_path => {}
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
                    directory_stats.maitainability_index = f64::min(
                        directory_stats.maitainability_index,
                        file_stats.maitainability_index,
                    );
                    directory_stats.changes_count += file_stats.changes_count;
                }
            }
        }

        self
    }

    fn collect_filenames(&mut self, path: &Path) -> &mut Self {
        let mut paths_to_visit = vec![path.to_path_buf()];

        while let Some(current_path) = paths_to_visit.pop() {
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

    pub fn collect_changes_count(&mut self, path: &Path) -> &mut Self {
        let output = Command::new("git")
            .current_dir(path)
            .arg("log")
            .arg("--name-only")
            .arg("--pretty=format:")
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
        let repo_base_path = Self::get_git_base_path(path);

        for line in lines {
            let filename_path = PathBuf::from(line);
            let absolute_path = repo_base_path.join(&filename_path);

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
            file_stats.maitainability_index = s.metrics.mi.mi_visual_studio();
        };
    }

    fn normalise_to_git_root(&mut self, path: &Path) -> &mut Self {
        let repo_base_path = Self::get_git_base_path(path);

        for (_, file_stats) in self.stats.iter_mut() {
            let path = Path::new(&file_stats.path).to_path_buf();
            let relative_path = path.strip_prefix(&repo_base_path).unwrap();
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
