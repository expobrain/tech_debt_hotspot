use chrono::NaiveDate;
use core::panic;
use rust_code_analysis::ParserTrait;
use rust_code_analysis::{metrics, PythonParser};
use std::path::PathBuf;
use std::{collections::HashMap, fs, path::Path, process::Command};
use tabled::Tabled;

#[derive(Clone, Default, Debug, PartialEq)]
struct FileStats {
    pub path: PathBuf,
    pub halstead_volume: f64,
    pub cyclomatic_complexity: f64,
    pub loc: u32,
    pub comments_percentage: f64,
    pub maintainability_index: f64,
    pub changes_count: u32,
}

#[derive(Tabled, Debug, PartialEq)]
pub struct HotspotStats {
    pub path: String,
    pub halstead_volume: f64,
    pub cyclomatic_complexity: f64,
    pub loc: u32,
    pub comments_percentage: f64,
    pub maintainability_index: f64,
    pub changes_count: u32,
    pub hotspot_index: f64,
}

impl HotspotStats {
    fn new(file_stats: &FileStats) -> HotspotStats {
        let hotspot_index = match file_stats.maintainability_index {
            0.0 => f64::INFINITY,
            _ => file_stats.changes_count as f64 / (file_stats.maintainability_index / 100.0),
        };

        HotspotStats {
            path: file_stats.path.display().to_string(),
            halstead_volume: file_stats.halstead_volume,
            cyclomatic_complexity: file_stats.cyclomatic_complexity,
            loc: file_stats.loc,
            comments_percentage: file_stats.comments_percentage,
            maintainability_index: file_stats.maintainability_index,
            changes_count: file_stats.changes_count,
            hotspot_index,
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
    pub fn new(directory: &Path, exclude: Option<&Path>, since: Option<&NaiveDate>) -> Self {
        Self {
            path: directory.to_path_buf(),
            exclude: exclude.map(|p| p.to_path_buf()),
            since: since.cloned(),
            git_base_path: Self::get_git_base_path(directory),
            ..Default::default()
        }
    }

    pub fn stats(&self) -> Vec<HotspotStats> {
        self.stats.values().map(HotspotStats::new).collect()
    }

    pub fn collect(&mut self) {
        self.collect_filenames()
            .get_stats_from_filenames()
            .collect_changes_count()
            .normalise_to_git_root();
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
            let sloc = s.metrics.loc.sloc();

            match sloc {
                0.0 => {
                    file_stats.maintainability_index = 100.0;
                    file_stats.comments_percentage = 0.0;
                    file_stats.halstead_volume = 0.0;
                }
                _ => {
                    file_stats.maintainability_index = s.metrics.mi.mi_visual_studio();
                    file_stats.comments_percentage = s.metrics.loc.cloc() / sloc * 100.0;
                    file_stats.halstead_volume = s.metrics.halstead.volume();
                }
            }

            file_stats.path = path;
            file_stats.cyclomatic_complexity = s.metrics.cyclomatic.cyclomatic_max();
            file_stats.loc = sloc as u32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use tempfile::{tempdir, TempDir};

    #[test]
    fn test_hotspot_stats_new() {
        // ARRANGE
        let file_stats = FileStats {
            path: PathBuf::from("src/main.rs"),
            halstead_volume: 10.0,
            cyclomatic_complexity: 5.0,
            loc: 100,
            comments_percentage: 20.0,
            maintainability_index: 80.0,
            changes_count: 10,
        };

        // ACT
        let actual = HotspotStats::new(&file_stats);

        // ASSERT
        let expected = HotspotStats {
            path: "src/main.rs".to_string(),
            halstead_volume: 10.0,
            cyclomatic_complexity: 5.0,
            loc: 100,
            comments_percentage: 20.0,
            maintainability_index: 80.0,
            changes_count: 10,
            hotspot_index: 10.0 / (80.0 / 100.0),
        };

        assert_eq!(actual, expected);
    }

    #[fixture]
    fn git_repo_with_files() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create a Git directory structure with some Python files
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        Command::new("git")
            .arg("init")
            .arg(temp_path)
            .output()
            .expect("Failed to initialize Git repository");

        let file1 = temp_path.join("file1.py");
        let file2 = sub_dir.join("file2.py");
        fs::write(&file1, "print('Hello, world!')").unwrap();
        fs::write(&file2, "print('Hello, subdir!')").unwrap();

        (temp_dir, file1.to_path_buf(), file2.to_path_buf())
    }
    #[rstest]
    fn test_collect_filenames(git_repo_with_files: (TempDir, PathBuf, PathBuf)) {
        // ARRANGE
        let (temp_dir, file1, file2) = git_repo_with_files;

        // ACT
        let mut tech_debt_hotspots = TechDebtHotspots::new(temp_dir.path(), None, None);
        tech_debt_hotspots.collect_filenames();

        let actual = tech_debt_hotspots.stats;

        // ASSERT
        let mut expected = HashMap::new();
        expected.insert(
            file1.clone(),
            FileStats {
                path: file1.clone(),
                ..Default::default()
            },
        );
        expected.insert(
            file2.clone(),
            FileStats {
                path: file2.clone(),
                ..Default::default()
            },
        );

        assert_eq!(actual, expected);
    }
}
