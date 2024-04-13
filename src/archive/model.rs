use crate::{utils::time::get_today_datetime, AResult, AppState};
use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, from_items, to_item};

use super::ARCHIVE_SK;

#[derive(Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub pk: String,
    pub sk: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    pub read_times: u64,
}

#[derive(Deserialize)]
pub struct ArchiveEntryFC {
    pub content: String,
    pub categories: Option<Vec<String>>,
}

impl From<ArchiveEntryFC> for ArchiveEntry {
    fn from(fc: ArchiveEntryFC) -> Self {
        let pk = String::from(ARCHIVE_SK);
        let sk = get_today_datetime();
        ArchiveEntry {
            pk,
            sk,
            content: fc.content,
            categories: fc.categories,
            read_times: 0,
        }
    }
}

impl ArchiveEntry {
    pub async fn ddb_increment_read_times(state: &AppState, sk: impl Into<String>) -> AResult<()> {
        let mut found_arch = match ArchiveEntry::ddb_find(state, sk).await {
            Ok(res) => res,
            Err(_) => {
                return Err(anyhow::Error::msg(
                    "ArchiveEntry with provided sort key does not exist",
                )
                .into())
            }
        };
        found_arch.read_times += 1;
        let item = to_item(found_arch)?;
        state
            .dynamodb_client
            .put_item()
            .table_name(&state.table_name)
            .set_item(Some(item))
            .send()
            .await?;
        Ok(())
    }

    pub async fn ddb_create(state: &AppState, record_fc: ArchiveEntryFC) -> AResult<()> {
        let item = to_item(Into::<ArchiveEntry>::into(record_fc))?;

        let req = state
            .dynamodb_client
            .put_item()
            .table_name(&state.table_name)
            .set_item(Some(item));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_find_all(state: AppState) -> AResult<Vec<ArchiveEntry>> {
        let query = state
            .dynamodb_client
            .query()
            .table_name(state.table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(":pk", AttributeValue::S(String::from(ARCHIVE_SK)));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let arch_entries: Vec<ArchiveEntry> = from_items(items)?;
                return Ok(arch_entries);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB Records. {:?}").into());
            }
        }
    }

    pub async fn ddb_find(state: &AppState, sk: impl Into<String>) -> AResult<ArchiveEntry> {
        let res = state
            .dynamodb_client
            .get_item()
            .table_name(&state.table_name)
            .key("pk", AttributeValue::S(String::from(ARCHIVE_SK)))
            .key("sk", AttributeValue::S(sk.into()))
            .send()
            .await?;

        let item = res
            .item
            .ok_or(anyhow::Error::msg("Error querying DynamoDB entries"))?;

        Ok(from_item(item)?)
    }

    pub async fn ddb_delete(state: &AppState, sk: impl Into<String>) -> AResult<()> {
        state
            .dynamodb_client
            .delete_item()
            .table_name(&state.table_name)
            .key("pk", AttributeValue::S(String::from(ARCHIVE_SK)))
            .key("sk", AttributeValue::S(sk.into()))
            .send()
            .await?;
        Ok(())
    }
}
