use std::{collections::HashMap, sync::Arc};

use app_database::entity::accounts::{AccountPlaceRef, AccountUserRef};
use serde::{Deserialize, Serialize};

pub mod info;
pub mod status;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRequest {
    pub info: info::WorkRequestInfo,
    pub request_id: Arc<str>,
    pub metadata: HashMap<String, String>,
    pub status: status::WorkRequestStatus,
    #[serde(default)]
    pub errors: Arc<[Arc<str>]>,
    #[serde(default)]
    pub parked: bool,
    #[serde(default)]
    pub ordered_by: Option<AccountUserRef>,
    #[serde(default)]
    pub ordered_in: Option<AccountPlaceRef>,
}
impl WorkRequest {
    #[must_use]
    #[inline]
    pub const fn info(&self) -> &info::WorkRequestInfo {
        &self.info
    }

    #[must_use]
    #[inline]
    pub fn request_id(&self) -> Arc<str> {
        self.request_id.clone()
    }

    #[must_use]
    #[inline]
    pub const fn status(&self) -> &status::WorkRequestStatus {
        &self.status
    }

    #[must_use]
    #[inline]
    pub const fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    #[must_use]
    #[inline]
    pub fn errors(&self) -> &[Arc<str>] {
        &self.errors
    }

    #[must_use]
    #[inline]
    pub const fn parked(&self) -> bool {
        self.parked
    }

    #[must_use]
    pub fn into_parts(self) -> (info::WorkRequestInfo, WorkRequestMeta) {
        (
            self.info,
            WorkRequestMeta {
                request_id: self.request_id,
                metadata: self.metadata,
                status: self.status,
                errors: self.errors,
                ordered_by: self.ordered_by,
                ordered_in: self.ordered_in,
            },
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRequestMeta {
    pub request_id: Arc<str>,
    pub metadata: HashMap<String, String>,
    pub status: status::WorkRequestStatus,
    #[serde(default)]
    pub errors: Arc<[Arc<str>]>,
    #[serde(default)]
    pub ordered_by: Option<AccountUserRef>,
    #[serde(default)]
    pub ordered_in: Option<AccountPlaceRef>,
}

impl WorkRequest {
    pub fn from_db_vec(
        vec: &[app_database::api::requests::RequestInfoResponse],
    ) -> Result<Arc<[Self]>, WorkRequestError> {
        vec.iter().map(std::convert::TryInto::try_into).collect()
    }
}

impl TryFrom<app_database::api::requests::RequestInfoResponse> for WorkRequest {
    type Error = WorkRequestError;

    fn try_from(
        value: app_database::api::requests::RequestInfoResponse,
    ) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl<'a> TryFrom<&'a app_database::api::requests::RequestInfoResponse> for WorkRequest {
    type Error = WorkRequestError;

    fn try_from(
        value: &'a app_database::api::requests::RequestInfoResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            info: value.info.clone().try_into()?,
            request_id: value.request_id.clone(),
            metadata: value.metadata.clone(),
            status: value.status.clone().try_into()?,
            errors: value.errors.clone(),
            parked: false,
            ordered_by: value.ordered_by.clone(),
            ordered_in: value.ordered_in.clone(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkRequestError {
    #[error("Invalid work request info: {0}")]
    InvalidWorkRequestInfo(#[from] info::WorkRequestInfoError),

    #[error("Invalid work request status: {0}")]
    InvalidWorkRequestStatus(#[from] status::WorkRequestStatusError),
}
