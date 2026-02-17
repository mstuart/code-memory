use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub enum LicenseStatus {
    Free,
    Pro { expires_at: u64, features: Vec<String> },
}

impl LicenseStatus {
    pub fn max_files(&self) -> usize {
        match self {
            LicenseStatus::Free => 5000,
            LicenseStatus::Pro { .. } => usize::MAX,
        }
    }

    pub fn allows_advanced_search(&self) -> bool {
        matches!(self, LicenseStatus::Pro { .. })
    }

    pub fn allows_team_features(&self) -> bool {
        matches!(self, LicenseStatus::Pro { .. })
    }
}

pub fn verify_license(key: &str) -> LicenseStatus {
    // Check cache first
    if let Some(cached) = check_cache(key) {
        return cached;
    }

    // Mock verification (will integrate API later)
    if key == "valid-pro-key-12345" {
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 86400 * 365;

        let status = LicenseStatus::Pro {
            expires_at,
            features: vec![
                "unlimited-files".to_string(),
                "advanced-search".to_string(),
                "team-features".to_string(),
            ],
        };

        save_to_cache(key, &status);
        return status;
    }

    LicenseStatus::Free
}

fn check_cache(key: &str) -> Option<LicenseStatus> {
    use crate::license::storage::load_cached_license_with_time;

    // Check if cache is fresh (24h)
    if let Some((status, cached_at)) = load_cached_license_with_time(key) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now - cached_at < 86400 {
            return Some(status);
        }
    }

    None
}

fn save_to_cache(key: &str, status: &LicenseStatus) {
    use crate::license::storage::save_cached_license;
    save_cached_license(key, status);
}
