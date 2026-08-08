use serde::{Deserialize, Serialize};

use crate::{Database, DatabaseError, DatabaseRequest, error::ResponseError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretEntry {
    pub name: String,
    pub value: String,
    #[serde(with = "crate::helpers::serde::bigint")]
    pub updated_at: u64,
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

    pub async fn secrets_set(&self, name: &str, value: &str) -> Result<(), DatabaseError> {
        DatabaseRequest::named("secrets:set")
            .with_arg("name", name)
            .with_arg("value", value)
            .mutate(self)
            .await
    }

    pub async fn secrets_remove(&self, name: &str) -> Result<(), DatabaseError> {
        DatabaseRequest::named("secrets:remove")
            .with_arg("name", name)
            .mutate(self)
            .await
    }
}
