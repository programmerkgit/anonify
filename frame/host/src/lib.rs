
pub mod ecalls;
mod error;
pub mod engine;
mod config;
pub mod init_enclave;
mod ocalls;

pub use error::FrameHostError as Error;
pub use init_enclave::EnclaveDir;
use std::{env, path::PathBuf};

lazy_static! {
    pub static ref PJ_ROOT_DIR: PathBuf = {
        let pj_root_dir = env::var("PJ_ROOT_DIR")
            .unwrap_or_else(|_| format!("{}/anonify", dirs::home_dir().unwrap().into_os_string().to_str().unwrap()));
        PathBuf::from(pj_root_dir)
    };
}
