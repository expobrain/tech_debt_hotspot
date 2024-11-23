use core::panic;
use rust_code_analysis::ParserTrait;
use rust_code_analysis::{metrics, PythonParser};
use std::path::PathBuf;
use std::{collections::HashMap, fs, path::Path, process::Command};

use crate::types::{FileStats, PathType};

fn get_stats_from_path(path: &Path) -> Result<Option<FileStats>, String> {
    let source_code = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let parser = PythonParser::new(source_code, path, None);
    let stats = metrics(&parser, path).map(|s| FileStats {
        path: path.to_string_lossy().to_string(),
        path_type: PathType::File,
        halstead_volume: s.metrics.halstead.volume(),
        cyclomatic_complexity: s.metrics.cyclomatic.cyclomatic_max(),
        loc: s.metrics.loc.sloc() as u32,
        comments_percentage: s.metrics.loc.cloc() / s.metrics.loc.sloc() * 100.0,
        maitainability_index: s.metrics.mi.mi_visual_studio(),
        ..Default::default()
    });

    Ok(stats)
}

pub fn collect_changes_count_from_path(
    path: &Path,
    stats: &mut HashMap<PathBuf, FileStats>,
) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(path)
        .arg("log")
        .arg("--name-only")
        .arg("--pretty=format:")
        .arg(".")
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Git command failed with status {}: {:?}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse git output: {}", e))?;
    let lines = stdout.lines().filter(|line| !line.trim().is_empty());
    let repo_base_path = get_git_base_path(path);

    for line in lines {
        let filename_path = PathBuf::from(line);
        let absolute_path = repo_base_path.join(&filename_path);

        if !absolute_path.exists()
            || absolute_path.extension().and_then(|s| s.to_str()) != Some("py")
        {
            continue;
        }

        // update filename stats
        match stats.get_mut(&absolute_path) {
            Some(existing) => {
                existing.changes_count += 1;
            }
            None => {
                let file_stats = FileStats {
                    path: absolute_path.to_string_lossy().to_string(),
                    path_type: PathType::File,
                    changes_count: 1,
                    ..Default::default()
                };

                stats.insert(absolute_path.clone(), file_stats);
            }
        };

        // update directory stats
        if let Some(parent) = absolute_path.parent() {
            let parent_path = parent.to_path_buf();

            match stats.get_mut(&parent_path) {
                Some(existing) => {
                    existing.changes_count += 1;
                }
                None => {
                    let file_stats = FileStats {
                        path: parent_path.to_string_lossy().to_string(),
                        path_type: PathType::Directory,
                        changes_count: 1,
                        ..Default::default()
                    };

                    stats.insert(parent_path, file_stats);
                }
            }
        };
    }

    Ok(())
}

pub fn update_stats_from_filename(filename: &Path, stats: &mut HashMap<PathBuf, FileStats>) {
    let files_stats = get_stats_from_path(filename).unwrap();

    if let Some(file_stats_data) = files_stats {
        // update filename stats
        match stats.get_mut(filename) {
            Some(existing) => {
                existing.halstead_volume = file_stats_data.halstead_volume;
                existing.cyclomatic_complexity = file_stats_data.cyclomatic_complexity;
                existing.loc = file_stats_data.loc;
                existing.comments_percentage = file_stats_data.comments_percentage;
                existing.maitainability_index = file_stats_data.maitainability_index;
            }
            None => {
                stats.insert(filename.into(), file_stats_data.clone());
            }
        }

        // update directory stats
        if let Some(parent) = filename.parent() {
            // let parent_path = parent.to_path_buf();

            match stats.get_mut(parent) {
                Some(existing) => {
                    existing.halstead_volume = existing
                        .halstead_volume
                        .max(file_stats_data.halstead_volume);
                    existing.cyclomatic_complexity = existing
                        .cyclomatic_complexity
                        .max(file_stats_data.cyclomatic_complexity);
                    existing.loc += file_stats_data.loc;
                    existing.comments_percentage = match existing.loc + file_stats_data.loc {
                        0 => 0.0,
                        _ => {
                            (existing.comments_percentage * f64::from(existing.loc)
                                + file_stats_data.comments_percentage
                                    * f64::from(file_stats_data.loc))
                                / f64::from(existing.loc + file_stats_data.loc)
                        }
                    };
                    existing.maitainability_index = existing
                        .maitainability_index
                        .min(file_stats_data.maitainability_index);
                }
                None => {
                    let mut path_stats = file_stats_data.clone();

                    path_stats.path = parent.to_string_lossy().to_string();
                    path_stats.path_type = PathType::Directory;

                    stats.insert(parent.into(), path_stats);
                }
            }
        };
    }
}

pub fn get_git_base_path(directory: &Path) -> PathBuf {
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

pub fn collect_stats_from_path(directory: &Path, stats: &mut HashMap<PathBuf, FileStats>) {
    let mut paths_to_visit = vec![directory.to_path_buf()];

    while let Some(current_path) = paths_to_visit.pop() {
        match current_path.is_dir() {
            true => {
                for entry in fs::read_dir(&current_path).unwrap() {
                    let path_to_visit = entry.unwrap().path();
                    paths_to_visit.push(path_to_visit);
                }
            }
            false if current_path.extension().and_then(|s| s.to_str()) == Some("py") => {
                update_stats_from_filename(&current_path, stats);
            }
            _ => {}
        }
    }
}
