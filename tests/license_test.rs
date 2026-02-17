use code_memory::license::{verify_license, LicenseStatus};

#[test]
fn test_valid_license_unlocks_pro() {
    let key = "valid-pro-key-12345";
    let status = verify_license(key);
    assert!(matches!(status, LicenseStatus::Pro { .. }));
}

#[test]
fn test_invalid_license_defaults_free() {
    let key = "invalid-key";
    let status = verify_license(key);
    assert!(matches!(status, LicenseStatus::Free));
}

#[test]
fn test_free_tier_has_5k_limit() {
    let status = LicenseStatus::Free;
    assert_eq!(status.max_files(), 5000);
}

#[test]
fn test_pro_tier_has_unlimited_files() {
    let status = LicenseStatus::Pro {
        expires_at: 9999999999,
        features: vec!["unlimited-files".to_string()],
    };
    assert_eq!(status.max_files(), usize::MAX);
}
