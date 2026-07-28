use serde::{Deserialize, Serialize};

use crate::{Database, DatabaseError, DatabaseRequest, error::ResponseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogSettingsScope {
    Global,
    Central,
    Worker,
    Bot,
    Admin,
}

impl LogSettingsScope {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Central => "central",
            Self::Worker => "worker",
            Self::Bot => "bot",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSettings {
    pub scope: LogSettingsScope,
    #[serde(default)]
    pub console: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveLogSettings {
    pub console: Option<String>,
    pub file: Option<String>,
}

#[must_use]
pub fn resolve_log_settings(
    settings: &[LogSettings],
    scope: LogSettingsScope,
) -> EffectiveLogSettings {
    let global = settings
        .iter()
        .find(|setting| setting.scope == LogSettingsScope::Global);
    let service = settings.iter().find(|setting| setting.scope == scope);

    EffectiveLogSettings {
        console: service
            .and_then(|setting| setting.console.clone())
            .or_else(|| global.and_then(|setting| setting.console.clone())),
        file: service
            .and_then(|setting| setting.file.clone())
            .or_else(|| global.and_then(|setting| setting.file.clone())),
    }
}

impl Database {
    pub async fn log_settings_list(&self) -> Result<Vec<LogSettings>, DatabaseError> {
        DatabaseRequest::named("logSettings:list").query(self).await
    }

    pub async fn log_settings_watch(
        &self,
    ) -> Result<impl futures::Stream<Item = Result<Vec<LogSettings>, ResponseError>>, DatabaseError>
    {
        DatabaseRequest::named("logSettings:list")
            .watch_query(self)
            .await
    }

    pub async fn log_settings_set(
        &self,
        scope: LogSettingsScope,
        console: Option<String>,
        file: Option<String>,
    ) -> Result<(), DatabaseError> {
        let mut request =
            DatabaseRequest::named("logSettings:set").with_arg("scope", scope.as_str());
        if let Some(console) = console {
            request = request.with_arg("console", console);
        }
        if let Some(file) = file {
            request = request.with_arg("file", file);
        }
        request.mutate(self).await
    }
}
