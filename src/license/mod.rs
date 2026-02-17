pub mod verify;
pub mod storage;

pub use verify::{verify_license, LicenseStatus};
pub use storage::{load_license_key, save_license_key};
