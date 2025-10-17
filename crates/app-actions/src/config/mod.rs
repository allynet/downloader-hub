use std::path::PathBuf;

use app_config::{
    common::{EndpointConfig, ProgramPathConfig, ProjectConfig},
    GlobalConfig,
};
use validator::Validate;

#[derive(Debug, Clone, Default, Validate, GlobalConfig)]
pub struct ActionsConfig {
    #[validate(nested)]
    pub endpoint: EndpointConfig,

    #[validate(nested)]
    pub dependency_paths: ProgramPathConfig,
}

impl ActionsConfig {
    #[must_use]
    #[inline]
    pub fn endpoints() -> &'static EndpointConfig {
        &Self::global().endpoint
    }

    #[must_use]
    #[inline]
    pub fn dependency_paths() -> &'static ProgramPathConfig {
        &Self::global().dependency_paths
    }

    #[must_use]
    #[inline]
    pub fn cache_dir() -> PathBuf {
        ProjectConfig::cache_dir()
    }
}
