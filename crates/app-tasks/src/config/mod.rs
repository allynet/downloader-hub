use std::ops::Deref;

use app_config::GlobalConfig;

#[derive(Debug, Clone, GlobalConfig)]
pub struct TaskConfig {
    pub conf: app_config::common::TaskConfig,
}

impl Deref for TaskConfig {
    type Target = app_config::common::TaskConfig;

    fn deref(&self) -> &Self::Target {
        &self.conf
    }
}
