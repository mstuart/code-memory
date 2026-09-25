use git2::{Oid, Repository, Sort};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::decisions::{Decision, DecisionExtractor};

/// Information about a single commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: i64,
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
}

/// History of a specific file across commits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistory {
    pub path: String,
    pub commits: Vec<CommitInfo>,
    pub decisions: Vec<Decision>,
    pub total_changes: usize,
    pub first_seen: i64,
    pub last_modified: i64,
}

/// Timeline entry for architectural decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTimeline {
    pub decisions: Vec<Decision>,
    pub files_affected: Vec<String>,
}

/// Main git history analyzer.
pub struct GitHistory {
    repo: Repository,
    extractor: DecisionExtractor,
}

fn blame_signature_details(
    signature: Option<git2::Signature<'_>>,
) -> Result<(String, i64), git2::Error> {
    let signature = signature
        .ok_or_else(|| git2::Error::from_str("blame hunk is missing its final signature"))?;
    let author = signature.name().unwrap_or("unknown").to_string();
    Ok((author, signature.when().seconds()))
}

impl GitHistory {
    /// Open a repository at the given path.
    pub fn open(repo_path: &Path) -> Result<Self, git2::Error> {
        let repo = Repository::open(repo_path)?;
        Ok(Self {
            repo,
            extractor: DecisionExtractor::new(),
        })
    }

    /// Discover a repository from any path within it.
    pub fn discover(path: &Path) -> Result<Self, git2::Error> {
        let repo = Repository::discover(path)?;
        Ok(Self {
            repo,
            extractor: DecisionExtractor::new(),
        })
    }

    /// Walk all commits in reverse chronological order, up to `max_commits`.
    pub fn walk_commits(&self, max_commits: usize) -> Result<Vec<CommitInfo>, git2::Error> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let mut commits = Vec::new();
        for (i, oid_result) in revwalk.enumerate() {
            if i >= max_commits {
                break;
            }
            let oid = oid_result?;
            if let Ok(info) = self.commit_info(oid) {
                commits.push(info);
            }
        }
        Ok(commits)
    }

    /// Get detailed info for a single commit.
    fn commit_info(&self, oid: Oid) -> Result<CommitInfo, git2::Error> {
        let commit = self.repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();

        let mut files_changed = Vec::new();
        let insertions;
        let deletions;

        let tree = commit.tree()?;
        if commit.parent_count() > 0 {
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let diff = self
                .repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;
            let stats = diff.stats()?;
            insertions = stats.insertions();
            deletions = stats.deletions();

            diff.foreach(
                &mut |delta, _| {
                    if let Some(path) = delta.new_file().path() {
                        files_changed.push(path.to_string_lossy().to_string());
                    }
                    true
                },
                None,
                None,
                None,
            )?;
        } else {
            let diff = self.repo.diff_tree_to_tree(None, Some(&tree), None)?;
            let stats = diff.stats()?;
            insertions = stats.insertions();
            deletions = stats.deletions();

            diff.foreach(
                &mut |delta, _| {
                    if let Some(path) = delta.new_file().path() {
                        files_changed.push(path.to_string_lossy().to_string());
                    }
                    true
                },
                None,
                None,
                None,
            )?;
        }

        Ok(CommitInfo {
            id: oid.to_string(),
            message,
            author: author.name().unwrap_or("unknown").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
            files_changed,
            insertions,
            deletions,
        })
    }

    /// Get the full history of a specific file.
    pub fn file_history(
        &self,
        file_path: &str,
        max_commits: usize,
    ) -> Result<FileHistory, git2::Error> {
        let mut histories = self.file_histories(&[file_path.to_string()], max_commits)?;
        Ok(histories.remove(file_path).unwrap_or_else(|| FileHistory {
            path: file_path.to_string(),
            commits: Vec::new(),
            decisions: Vec::new(),
            total_changes: 0,
            first_seen: 0,
            last_modified: 0,
        }))
    }

    /// Get histories for several repository-relative paths with one commit traversal.
    pub fn file_histories(
        &self,
        file_paths: &[String],
        max_commits: usize,
    ) -> Result<HashMap<String, FileHistory>, git2::Error> {
        let mut histories: HashMap<String, FileHistory> = file_paths
            .iter()
            .map(|path| {
                (
                    path.clone(),
                    FileHistory {
                        path: path.clone(),
                        commits: Vec::new(),
                        decisions: Vec::new(),
                        total_changes: 0,
                        first_seen: 0,
                        last_modified: 0,
                    },
                )
            })
            .collect();

        if histories.is_empty() || max_commits == 0 {
            return Ok(histories);
        }

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        for oid_result in revwalk {
            let oid = oid_result?;
            if let Ok(info) = self.commit_info(oid) {
                let matching_paths: Vec<String> = histories
                    .iter()
                    .filter(|(path, history)| {
                        history.commits.len() < max_commits
                            && info.files_changed.iter().any(|changed| changed == *path)
                    })
                    .map(|(path, _)| path.clone())
                    .collect();
                if matching_paths.is_empty() {
                    continue;
                }
                let commit_decisions = self.extractor.extract(&info);
                for path in matching_paths {
                    if let Some(history) = histories.get_mut(&path) {
                        history.decisions.extend(commit_decisions.iter().cloned());
                        history.commits.push(info.clone());
                    }
                }
                if histories
                    .values()
                    .all(|history| history.commits.len() >= max_commits)
                {
                    break;
                }
            }
        }

        for history in histories.values_mut() {
            history.first_seen = history.commits.last().map(|c| c.timestamp).unwrap_or(0);
            history.last_modified = history.commits.first().map(|c| c.timestamp).unwrap_or(0);
            history.total_changes = history.commits.len();
        }

        Ok(histories)
    }

    /// Translate index-root-relative paths and load their histories in one traversal.
    pub fn file_histories_from_root(
        &self,
        project_root: &Path,
        file_paths: &[String],
        max_commits: usize,
    ) -> Result<HashMap<String, FileHistory>, git2::Error> {
        let mappings: HashMap<String, String> = file_paths
            .iter()
            .filter_map(|path| {
                self.repository_relative_path(&project_root.join(path))
                    .map(|repo_path| (path.clone(), repo_path))
            })
            .collect();
        let repo_paths: Vec<String> = mappings.values().cloned().collect();
        let repo_histories = self.file_histories(&repo_paths, max_commits)?;

        Ok(mappings
            .into_iter()
            .filter_map(|(index_path, repo_path)| {
                repo_histories.get(&repo_path).cloned().map(|mut history| {
                    history.path.clone_from(&index_path);
                    (index_path, history)
                })
            })
            .collect())
    }

    fn repository_relative_path(&self, path: &Path) -> Option<String> {
        let workdir = self.repo.workdir()?;
        let canonical_workdir =
            std::fs::canonicalize(workdir).unwrap_or_else(|_| PathBuf::from(workdir));
        let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
        canonical_path
            .strip_prefix(canonical_workdir)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
    }

    /// Extract all decisions from the repository's commit history.
    pub fn extract_all_decisions(
        &self,
        max_commits: usize,
    ) -> Result<DecisionTimeline, git2::Error> {
        let commits = self.walk_commits(max_commits)?;
        let mut all_decisions = Vec::new();
        let mut files_affected = Vec::new();

        for commit in &commits {
            let extracted = self.extractor.extract(commit);
            if !extracted.is_empty() {
                for file in &commit.files_changed {
                    if !files_affected.contains(file) {
                        files_affected.push(file.clone());
                    }
                }
                all_decisions.extend(extracted);
            }
        }

        Ok(DecisionTimeline {
            decisions: all_decisions,
            files_affected,
        })
    }

    /// Get a map of file paths to their change frequency.
    pub fn file_change_frequency(
        &self,
        max_commits: usize,
    ) -> Result<HashMap<String, usize>, git2::Error> {
        let commits = self.walk_commits(max_commits)?;
        let mut freq: HashMap<String, usize> = HashMap::new();

        for commit in &commits {
            for file in &commit.files_changed {
                *freq.entry(file.clone()).or_insert(0) += 1;
            }
        }

        Ok(freq)
    }

    /// Get recently modified files.
    pub fn recent_files(&self, limit: usize) -> Result<Vec<(String, i64)>, git2::Error> {
        let commits = self.walk_commits(limit * 3)?;
        let mut seen = HashMap::new();

        for commit in &commits {
            for file in &commit.files_changed {
                seen.entry(file.clone()).or_insert(commit.timestamp);
            }
        }

        let mut files: Vec<(String, i64)> = seen.into_iter().collect();
        files.sort_by_key(|b| std::cmp::Reverse(b.1));
        files.truncate(limit);
        Ok(files)
    }

    /// Get the blame for a specific file.
    pub fn file_blame(&self, file_path: &str) -> Result<Vec<(String, String, i64)>, git2::Error> {
        let blame = self.repo.blame_file(Path::new(file_path), None)?;
        let mut result = Vec::new();

        for hunk in blame.iter() {
            let (author, timestamp) = blame_signature_details(hunk.final_signature())?;
            let commit_id = hunk.final_commit_id().to_string();
            result.push((author, commit_id, timestamp));
        }

        Ok(result)
    }

    /// Get PR references from commit messages.
    pub fn extract_pr_references(
        &self,
        max_commits: usize,
    ) -> Result<Vec<(String, Vec<String>)>, git2::Error> {
        let commits = self.walk_commits(max_commits)?;
        let pr_pattern = regex::Regex::new(r"#(\d+)|(?:pull request|PR)\s*#?(\d+)").unwrap();

        let mut results = Vec::new();
        for commit in &commits {
            let pr_refs: Vec<String> = pr_pattern
                .captures_iter(&commit.message)
                .filter_map(|cap| {
                    cap.get(1)
                        .or_else(|| cap.get(2))
                        .map(|m| format!("#{}", m.as_str()))
                })
                .collect();

            if !pr_refs.is_empty() {
                results.push((commit.id.clone(), pr_refs));
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();

        Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&path)
            .output()
            .unwrap();

        std::fs::write(path.join("test.rs"), "fn main() {}").unwrap();
        std::fs::create_dir_all(path.join("packages/app/src")).unwrap();
        std::fs::write(
            path.join("packages/app/src/nested.rs"),
            "pub fn nested() {}",
        )
        .unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "commit",
                "-m",
                "decision: chose Rust over Go for performance",
            ])
            .current_dir(&path)
            .output()
            .unwrap();

        std::fs::write(path.join("lib.rs"), "pub fn hello() {}").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "commit",
                "-m",
                "Add library module\n\nrationale: separate concerns",
            ])
            .current_dir(&path)
            .output()
            .unwrap();

        (dir, path)
    }

    #[test]
    fn test_walk_commits() {
        let (_dir, path) = setup_test_repo();
        let git = GitHistory::open(&path).unwrap();
        let commits = git.walk_commits(100).unwrap();
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn test_file_history() {
        let (_dir, path) = setup_test_repo();
        let git = GitHistory::open(&path).unwrap();
        let history = git.file_history("test.rs", 100).unwrap();
        assert_eq!(history.path, "test.rs");
        assert_eq!(history.commits.len(), 1);
    }

    #[test]
    fn test_file_histories_from_nested_project_root() {
        let (_dir, path) = setup_test_repo();
        let git = GitHistory::open(&path).unwrap();
        let histories = git
            .file_histories_from_root(
                &path.join("packages/app"),
                &[String::from("src/nested.rs")],
                3,
            )
            .unwrap();
        let history = histories.get("src/nested.rs").unwrap();
        assert_eq!(history.commits.len(), 1);
        assert!(history.commits[0]
            .files_changed
            .contains(&String::from("packages/app/src/nested.rs")));
    }

    #[test]
    fn test_extract_decisions() {
        let (_dir, path) = setup_test_repo();
        let git = GitHistory::open(&path).unwrap();
        let timeline = git.extract_all_decisions(100).unwrap();
        assert!(!timeline.decisions.is_empty());
    }

    #[test]
    fn test_file_change_frequency() {
        let (_dir, path) = setup_test_repo();
        let git = GitHistory::open(&path).unwrap();
        let freq = git.file_change_frequency(100).unwrap();
        assert_eq!(freq.get("test.rs"), Some(&1));
    }

    #[test]
    fn test_missing_blame_signature_returns_error() {
        assert!(blame_signature_details(None).is_err());
    }
}
