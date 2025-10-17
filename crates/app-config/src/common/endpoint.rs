use clap::{Args, ValueHint};
use serde::{Deserialize, Serialize};
use url::Url;
use validator::Validate;

use crate::validators::url::{
    validate_is_absolute_url, value_parser_parse_absolute_url,
    value_parser_parse_absolute_url_as_url,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("External endpoints/APIs"))]
pub struct EndpointConfig {
    /// The base URL for the Twitter screenshot API.
    #[arg(long, default_value = "https://twitter.igr.ec", env = "DOWNLOADER_HUB_ENDPOINT_TWITTER_SCREENSHOT", value_hint = ValueHint::Url, value_parser = value_parser_parse_absolute_url())]
    #[validate(custom(function = "validate_is_absolute_url"))]
    pub twitter_screenshot_base_url: String,

    /// The base URL for the OCR API.
    #[arg(long, env = "DOWNLOADER_HUB_ENDPOINT_OCR_API", value_hint = ValueHint::Url, value_parser = value_parser_parse_absolute_url_as_url())]
    pub ocr_api_base_url: Option<Url>,
}
impl EndpointConfig {
    #[must_use]
    pub fn ocr_api_url(&self, path: &str) -> Option<Url> {
        self.ocr_api_base_url
            .as_ref()
            .and_then(|x| x.join(path.trim_start_matches('/')).ok())
    }
}
