use serde::{Deserialize, Serialize};

use crate::{
    Database, DatabaseError, DatabaseRequest,
    api::accounts::{place_ref_value, user_ref_value},
    entity::accounts::{AccountPlaceRef, AccountUserRef},
    error::ResponseError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretEntry {
    pub name: String,
    pub value: String,
    #[serde(with = "crate::helpers::serde::bigint")]
    pub updated_at: u64,
    #[serde(default)]
    pub allowed_users: Vec<AccountUserRef>,
    #[serde(default)]
    pub allowed_places: Vec<AccountPlaceRef>,
}

impl Database {
    pub async fn secrets_list(&self) -> Result<Vec<SecretEntry>, DatabaseError> {
        DatabaseRequest::named("secrets:list").query(self).await
    }

    pub async fn secrets_watch(
        &self,
    ) -> Result<impl futures::Stream<Item = Result<Vec<SecretEntry>, ResponseError>>, DatabaseError>
    {
        DatabaseRequest::named("secrets:list")
            .watch_query(self)
            .await
    }

    pub async fn secrets_set(
        &self,
        name: &str,
        value: &str,
        allowed_users: &[AccountUserRef],
        allowed_places: &[AccountPlaceRef],
    ) -> Result<(), DatabaseError> {
        let mut req = DatabaseRequest::named("secrets:set")
            .with_arg("name", name)
            .with_arg("value", value);
        if !allowed_users.is_empty() {
            let arr: Vec<convex::Value> = allowed_users.iter().map(user_ref_value).collect();
            req = req.with_arg("allowedUsers", convex::Value::Array(arr));
        }
        if !allowed_places.is_empty() {
            let arr: Vec<convex::Value> = allowed_places.iter().map(place_ref_value).collect();
            req = req.with_arg("allowedPlaces", convex::Value::Array(arr));
        }
        req.mutate(self).await
    }

    pub async fn secrets_remove(&self, name: &str) -> Result<(), DatabaseError> {
        DatabaseRequest::named("secrets:remove")
            .with_arg("name", name)
            .mutate(self)
            .await
    }
}
