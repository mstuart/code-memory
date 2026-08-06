use git2::{Oid, Repository, Sort};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let mut commits = Vec::new();
        let mut decisions = Vec::new();

        for oid_result in revwalk {
            let oid = oid_result?;
            if let Ok(info) = self.commit_info(oid) {
                if info.files_changed.iter().any(|f| f == file_path) {
                    let commit_decisions = self.extractor.extract(&info);
                    decisions.extend(commit_decisions);
                    commits.push(info);

                    if commits.len() >= max_commits {
                        break;
                    }
                }
            }
        }

        let first_seen = commits.last().map(|c| c.timestamp).unwrap_or(0);
        let last_modified = commits.first().map(|c| c.timestamp).unwrap_or(0);
        let total_changes = commits.len();

        Ok(FileHistory {
            path: file_path.to_string(),
            commits,
            decisions,
            total_changes,
            first_seen,
            last_modified,
        })
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
        files.sort_by(|a, b| b.1.cmp(&a.1));
        files.truncate(limit);
        Ok(files)
    }

    /// Get the blame for a specific file.
    pub fn file_blame(&self, file_path: &str) -> Result<Vec<(String, String, i64)>, git2::Error> {
        let blame = self.repo.blame_file(Path::new(file_path), None)?;
        let mut result = Vec::new();

        for hunk in blame.iter() {
            let sig = hunk.final_signature();
            let author = sig.name().unwrap_or("unknown").to_string();
            let commit_id = hunk.final_commit_id().to_string();
            let timestamp = sig.when().seconds();
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
}
