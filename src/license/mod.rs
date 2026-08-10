pub mod storage;
pub mod verify;

pub use storage::{load_license_key, save_license_key};
pub use verify::{verify_license, LicenseStatus};
