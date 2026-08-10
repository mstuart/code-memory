use crate::license::verify::LicenseStatus;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct CachedLicense {
    key: String,
    tier: String,
    expires_at: Option<u64>,
    features: Option<Vec<String>>,
    cached_at: u64,
}

pub fn load_license_key() -> Option<String> {
    let path = get_license_path();
    fs::read_to_string(&path).ok()
}

pub fn save_license_key(key: &str) -> std::io::Result<()> {
    let path = get_license_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, key)
}

pub fn load_cached_license_with_time(key: &str) -> Option<(LicenseStatus, u64)> {
    let path = get_cache_path();
    let content = fs::read_to_string(&path).ok()?;
    let cached: CachedLicense = serde_json::from_str(&content).ok()?;

    if cached.key != key {
        return None;
    }

    let status = match cached.tier.as_str() {
        "pro" => LicenseStatus::Pro {
            expires_at: cached.expires_at?,
            features: cached.features?,
        },
        _ => LicenseStatus::Free,
    };

    Some((status, cached.cached_at))
}

pub fn save_cached_license(key: &str, status: &LicenseStatus) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let (tier, expires_at, features) = match status {
        LicenseStatus::Pro {
            expires_at,
            features,
        } => ("pro", Some(*expires_at), Some(features.clone())),
        LicenseStatus::Free => ("free", None, None),
    };

    let cached = CachedLicense {
        key: key.to_string(),
        tier: tier.to_string(),
        expires_at,
        features,
        cached_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let path = get_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, serde_json::to_string(&cached).unwrap());
}

fn get_license_path() -> PathBuf {
    dirs::config_dir()
        .unwrap()
        .join("code-memory")
        .join("license.key")
}

fn get_cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap()
        .join("code-memory")
        .join("license_cache.json")
}
