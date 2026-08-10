use crate::license::LicenseStatus;

pub fn check_file_limit(current_count: usize, license: &LicenseStatus) -> Result<(), String> {
    let max_files = license.max_files();

    if current_count > max_files {
        return Err(format!(
            "File limit exceeded. Current: {}, Max: {} (Free tier). Upgrade to Pro for unlimited files: https://code-memory.com/pro",
            current_count, max_files
        ));
    }

    Ok(())
}

pub fn get_file_count(index_path: &std::path::Path) -> Result<usize, String> {
    use tantivy::Index;

    let index =
        Index::open_in_dir(index_path).map_err(|e| format!("Failed to open index: {}", e))?;

    let reader = index
        .reader()
        .map_err(|e| format!("Failed to create reader: {}", e))?;

    let searcher = reader.searcher();
    Ok(searcher.num_docs() as usize)
}
