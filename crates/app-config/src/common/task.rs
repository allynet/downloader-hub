use clap::Args;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::timeframe::Timeframe;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("Task options"))]
pub struct TaskConfig {
    /// The interval at which the bot should check for updates to the yt-dlp binary.
    /// If not set, the bot will not check for updates.
    ///
    /// The value represents a duration in seconds, minutes, hours, days, weeks, or months.
    /// Special events are ignored, eg. leap years, daylight savings, etc.
    /// `minute` is 60 seconds, `hour` is 60 minutes, `day` is 24 hours, `week` is 7 days, `month` is 30 days.
    /// Eg. 1d, 2 weeks, 3 months, 4h, 5mins, 6s
    #[clap(short, long, value_parser = Timeframe::parse_str, env = "DOWNLOADER_HUB_YT_DLP_UPDATE_INTERVAL")]
    pub yt_dlp_update_interval: Option<Timeframe>,
}
