use chrono::NaiveDate;
use core::panic;
use rust_code_analysis::{get_function_spaces, LANG};
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

fn extension_to_lang(ext: &str) -> Option<LANG> {
    match ext {
        "py" => Some(LANG::Python),
        "js" | "mjs" | "jsx" => Some(LANG::Javascript),
        "ts" => Some(LANG::Typescript),
        "tsx" => Some(LANG::Tsx),
        _ => None,
    }
}

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .and_then(extension_to_lang)
        .is_some()
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
                false if is_supported_file(&current_path) => {
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
            command.arg(format!("--since={since}"));
        }

        let output = command
            .arg(".")
            .output()
            .map_err(|e| format!("Failed to execute git command: {e}"))
            .unwrap();

        if !output.status.success() {
            panic!(
                "Git command failed with status {}: {:?}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse git output: {e}"))
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
        for file_stats in self.stats.values_mut() {
            Self::get_stats_from_filename(file_stats);
        }

        self
    }

    fn get_stats_from_filename(file_stats: &mut FileStats) {
        let path = Path::new(&file_stats.path).to_path_buf();
        let source_code = fs::read(path.clone()).unwrap();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let lang = extension_to_lang(ext).unwrap();

        if let Some(s) = get_function_spaces(&lang, source_code, &path, None) {
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
        for file_stats in self.stats.values_mut() {
            let path = Path::new(&file_stats.path).to_path_buf();
            let relative_path = path.strip_prefix(&self.git_base_path);

            match relative_path {
                Ok(relative_path) => file_stats.path = relative_path.to_path_buf(),
                Err(_) => panic!("Path is not in the Git repository"),
            }
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
    fn git_repo_with_files() -> (TempDir, Vec<PathBuf>) {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        Command::new("git")
            .arg("init")
            .arg(temp_path)
            .output()
            .expect("Failed to initialize Git repository");

        let py_file = temp_path.join("file1.py");
        let js_file = sub_dir.join("file2.js");
        let ts_file = temp_path.join("file3.ts");
        let tsx_file = sub_dir.join("file4.tsx");
        let txt_file = temp_path.join("readme.txt");

        fs::write(&py_file, "print('Hello, world!')").unwrap();
        fs::write(&js_file, "const x = 1;").unwrap();
        fs::write(&ts_file, "const x: number = 1;").unwrap();
        fs::write(&tsx_file, "const x: number = 1;").unwrap();
        fs::write(&txt_file, "just text").unwrap();

        (temp_dir, vec![py_file, js_file, ts_file, tsx_file])
    }

    #[rstest]
    fn test_collect_filenames(git_repo_with_files: (TempDir, Vec<PathBuf>)) {
        // ARRANGE
        let (temp_dir, supported_files) = git_repo_with_files;

        // ACT
        let mut tech_debt_hotspots = TechDebtHotspots::new(temp_dir.path(), None, None);
        tech_debt_hotspots.collect_filenames();

        let actual = tech_debt_hotspots.stats;

        // ASSERT
        assert_eq!(actual.len(), 4);

        for path in &supported_files {
            assert!(actual.contains_key(path), "Missing: {}", path.display());
        }
    }

    #[rstest]
    fn test_extension_to_lang_supported(
        #[values("py", "js", "mjs", "jsx", "ts", "tsx")] ext: &str,
    ) {
        assert!(
            super::extension_to_lang(ext).is_some(),
            "Expected {ext} to map to a language"
        );
    }

    #[rstest]
    fn test_extension_to_lang_unknown(#[values("txt", "md", "", "rs", "java", "cpp")] ext: &str) {
        assert!(
            super::extension_to_lang(ext).is_none(),
            "Expected {ext} to not map to any language"
        );
    }

    #[rstest]
    fn test_get_stats_from_filename_parses_supported_languages(
        #[values("py", "js", "ts", "tsx")] ext: &str,
    ) {
        let temp_dir = tempdir().unwrap();
        let source = match ext {
            "py" => "x = 1\ny = 2\nprint(x + y)\n",
            "js" => "const x = 1;\nconst y = 2;\nconsole.log(x + y);\n",
            "ts" => "const x: number = 1;\nconst y: number = 2;\nconsole.log(x + y);\n",
            "tsx" => "const x: number = 1;\nconst y: number = 2;\nconsole.log(x + y);\n",
            _ => "",
        };

        let path = temp_dir.path().join(format!("test.{ext}"));
        fs::write(&path, source).unwrap();

        let mut file_stats = FileStats {
            path: path.clone(),
            ..Default::default()
        };

        TechDebtHotspots::get_stats_from_filename(&mut file_stats);

        assert!(
            file_stats.loc > 0,
            "Expected loc > 0 for .{ext}, got {}",
            file_stats.loc
        );
        assert!(
            file_stats.maintainability_index > 0.0,
            "Expected maintainability_index > 0 for .{ext}, got {}",
            file_stats.maintainability_index
        );
    }

    #[rstest]
    fn test_normalise_to_git_root() {
        // ARRANGE
        let temp_dir = tempdir().unwrap();
        let git_base_path = temp_dir.path().to_path_buf();
        let file_path = git_base_path.join("src/main.py");

        // Create a TechDebtHotspots instance
        let mut tech_debt_hotspots = TechDebtHotspots {
            git_base_path: git_base_path.clone(),
            stats: HashMap::new(),
            path: git_base_path.clone(),
            exclude: None,
            since: None,
        };

        // Insert a FileStats entry with an absolute path
        tech_debt_hotspots.stats.insert(
            file_path.clone(),
            FileStats {
                path: file_path.clone(),
                ..Default::default()
            },
        );

        // ACT
        tech_debt_hotspots.normalise_to_git_root();

        // ASSERT
        let normalized_path = tech_debt_hotspots
            .stats
            .get(&file_path)
            .unwrap()
            .path
            .clone();
        let expected_relative_path = file_path
            .strip_prefix(&git_base_path)
            .unwrap()
            .to_path_buf();

        assert_eq!(normalized_path, expected_relative_path);
    }
}
