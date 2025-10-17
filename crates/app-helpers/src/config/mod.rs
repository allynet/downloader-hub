use std::path::PathBuf;

use app_config::{
    common::{ProgramPathConfig, ProjectConfig},
    GlobalConfig,
};
use validator::Validate;

#[derive(Debug, Clone, Default, Validate, GlobalConfig)]
pub struct HelpersConfig {
    /// Path to various programs used by the application at runtime
    #[validate(nested)]
    pub dependency_paths: ProgramPathConfig,
}

impl HelpersConfig {
    #[must_use]
    #[inline]
    pub fn get_cache_dir(&self) -> PathBuf {
        ProjectConfig::cache_dir()
    }

    #[must_use]
    #[inline]
    pub fn dependency_paths() -> &'static ProgramPathConfig {
        &Self::global().dependency_paths
    }
}
