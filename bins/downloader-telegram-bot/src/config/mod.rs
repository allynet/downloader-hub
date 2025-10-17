use app_config::{
    common, conditional::telegram_bot::TelegramBotConfig as BotConfig,
    validators::print_validation_errors, Dumpable, GlobalConfig,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use validator::Validate;

mod add;

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, Parser, Validate, GlobalConfig, Dumpable,
)]
pub struct Config {
    #[clap(flatten)]
    #[validate(nested)]
    pub bot: BotConfig,

    #[clap(flatten)]
    #[validate(nested)]
    pub dependency_paths: common::ProgramPathConfig,

    #[clap(flatten)]
    #[validate(nested)]
    pub endpoint: common::EndpointConfig,

    #[clap(flatten)]
    #[validate(nested)]
    pub task: common::TaskConfig,

    #[clap(flatten)]
    #[validate(nested)]
    #[serde(skip)]
    dump: DumpConfig,
}

impl Config {
    pub fn init_parsed() -> Result<&'static Self, String> {
        let parsed = Self::parse()
            .resolve_paths()
            .validate_or_exit()
            .dump_if_needed();

        {
            let parsed = parsed.clone();
            app_actions::config::ActionsConfig::init(app_actions::config::ActionsConfig {
                endpoint: parsed.endpoint,
                dependency_paths: parsed.dependency_paths,
            })?;
        }

        {
            let parsed = parsed.clone();
            app_helpers::config::HelpersConfig::init(app_helpers::config::HelpersConfig {
                dependency_paths: parsed.dependency_paths,
            })?;
        }

        {
            let parsed = parsed.clone();
            app_tasks::config::TaskConfig::init(app_tasks::config::TaskConfig {
                conf: parsed.task,
            })?;
        }

        Self::init(parsed)
    }

    #[must_use]
    #[inline]
    pub fn bot() -> &'static BotConfig {
        &Self::global().bot
    }

    #[inline]
    fn resolve_paths(mut self) -> Self {
        self.dependency_paths = self.dependency_paths.resolve_paths();
        self
    }

    #[inline]
    fn validate_or_exit(self) -> Self {
        if let Err(e) = self.validate() {
            eprintln!("Errors validating configuration:");
            print_validation_errors(&e, "  ", 1);
            std::process::exit(1);
        }

        self
    }
}
