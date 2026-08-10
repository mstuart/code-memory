use code_memory::indexer::limits::check_file_limit;
use code_memory::license::LicenseStatus;

#[test]
fn test_free_tier_blocks_over_5k_files() {
    let status = LicenseStatus::Free;
    let current_files = 5001;

    let result = check_file_limit(current_files, &status);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("5000"));
}

#[test]
fn test_free_tier_allows_under_5k_files() {
    let status = LicenseStatus::Free;
    let current_files = 4999;

    let result = check_file_limit(current_files, &status);
    assert!(result.is_ok());
}

#[test]
fn test_pro_tier_allows_unlimited_files() {
    let status = LicenseStatus::Pro {
        expires_at: 9999999999,
        features: vec!["unlimited-files".to_string()],
    };
    let current_files = 50000;

    let result = check_file_limit(current_files, &status);
    assert!(result.is_ok());
}
