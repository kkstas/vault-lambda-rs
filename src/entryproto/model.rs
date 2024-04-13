use crate::{AResult, AppState};
use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, from_items, to_item};

#[derive(Serialize, Deserialize)]
pub struct EntryProto {
    pub pk: String, // "EntryProto::Active" || "EntryProto::Inactive"
    pub sk: String, // Primary key of referenced entry, e.g. "Entry::Dream"
    pub title: String,
}

#[derive(Deserialize)]
pub struct EntryProtoFC {
    pub sk: String,
    pub title: String,
}

impl From<EntryProtoFC> for EntryProto {
    fn from(entry_proto_fc: EntryProtoFC) -> Self {
        EntryProto {
            pk: String::from("EntryProto::Active"),
            sk: entry_proto_fc.sk,
            title: entry_proto_fc.title,
        }
    }
}

impl EntryProto {
    pub async fn set_as_inactive(state: &AppState, sk: impl Into<String>) -> AResult<()> {
        let sk = sk.into();
        if !(sk.starts_with("Entry::")) {
            return Err(anyhow::Error::msg("Invalid EntryProto sort key").into());
        }

        let active_query_res = match EntryProto::ddb_find(state, "EntryProto::Active", &sk).await {
            Ok(res) => res,
            Err(_) => {
                return Err(anyhow::Error::msg(
                    "Active EntryProto with provided sort key does not exist",
                )
                .into())
            }
        };

        if EntryProto::ddb_find(state, "EntryProto::Inactive", &sk)
            .await
            .is_ok()
        {
            return Err(
                anyhow::Error::msg("EntryProto already exists in DynamoDB as inactive").into(),
            );
        }

        let entry = EntryProto {
            pk: String::from("EntryProto::Inactive"),
            sk: sk.clone(),
            title: active_query_res.title.clone(),
        };
        let item = to_item(entry)?;
        state
            .dynamodb_client
            .put_item()
            .table_name(&state.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        EntryProto::ddb_delete(state, "EntryProto::Active", sk).await?;
        Ok(())
    }

    pub async fn set_as_active(state: &AppState, sk: impl Into<String>) -> AResult<()> {
        let sk = sk.into();
        if !(sk.starts_with("Entry::")) {
            return Err(anyhow::Error::msg("Invalid EntryProto sort key").into());
        }

        if EntryProto::ddb_find(state, "EntryProto::Active", &sk)
            .await
            .is_ok()
        {
            return Err(
                anyhow::Error::msg("EntryProto already exists in DynamoDB as active").into(),
            );
        }

        let inactive_entry = match EntryProto::ddb_find(state, "EntryProto::Inactive", &sk).await {
            Ok(res) => res,
            Err(_) => {
                return Err(
                    anyhow::Error::msg("EntryProto does not exist in DynamoDB as inactive").into(),
                )
            }
        };

        let entry = EntryProto {
            pk: String::from("EntryProto::Active"),
            sk: sk.clone(),
            title: inactive_entry.title.clone(),
        };

        let item = to_item(entry)?;
        state
            .dynamodb_client
            .put_item()
            .table_name(&state.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        EntryProto::ddb_delete(state, "EntryProto::Inactive", sk).await?;
        Ok(())
    }

    pub async fn ddb_put_item(state: &AppState, entry_proto_fc: EntryProtoFC) -> AResult<()> {
        if EntryProto::ddb_find(state, "EntryProto::Inactive", &entry_proto_fc.sk)
            .await
            .is_ok()
        {
            return Err(
                anyhow::Error::msg("EntryProto already exists in DynamoDB as inactive").into(),
            );
        }

        let entry = EntryProto {
            pk: String::from("EntryProto::Active"),
            sk: entry_proto_fc.sk.clone(),
            title: entry_proto_fc.title,
        };
        let item = to_item(entry)?;
        state
            .dynamodb_client
            .put_item()
            .table_name(&state.table_name)
            .set_item(Some(item))
            .send()
            .await?;
        Ok(())
    }

    pub async fn ddb_delete(
        state: &AppState,
        pk: impl Into<String>,
        sk: impl Into<String>,
    ) -> AResult<()> {
        let pk = pk.into();
        if !pk.starts_with("EntryProto::") {
            return Err(anyhow::Error::msg("Invalid EntryProto primary key").into());
        }
        state
            .dynamodb_client
            .delete_item()
            .table_name(&state.table_name)
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk.into()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn ddb_list_active(state: &AppState) -> AResult<Vec<EntryProto>> {
        let query = state
            .dynamodb_client
            .query()
            .table_name(&state.table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S("EntryProto::Active".to_string()),
            );

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let entries: Vec<EntryProto> = from_items(items)?;
                return Ok(entries);
            }
            None => {
                return Err(anyhow::Error::msg("Error listing DynamoDB active EntryProto").into());
            }
        }
    }

    pub async fn ddb_find(
        state: &AppState,
        pk: impl Into<String>,
        sk: impl Into<String>,
    ) -> AResult<EntryProto> {
        let pk = pk.into();
        let sk = sk.into();
        if (pk != "EntryProto::Active") && (pk != "EntryProto::Inactive") {
            return Err(
                anyhow::Error::msg("Invalid EntryProto query partition key argument").into(),
            );
        }

        if !sk.starts_with("Entry::") {
            return Err(anyhow::Error::msg("Invalid EntryProto query sort key argument").into());
        }
        let res = state
            .dynamodb_client
            .get_item()
            .table_name(&state.table_name)
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk))
            .send()
            .await?;

        let item = res
            .item
            .ok_or(anyhow::Error::msg("Error querying DynamoDB entries"))?;

        Ok(from_item(item)?)
    }

    pub async fn ddb_list_inactive(state: &AppState) -> AResult<Vec<EntryProto>> {
        let query = state
            .dynamodb_client
            .query()
            .table_name(&state.table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S("EntryProto::Inactive".to_string()),
            );

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let entries: Vec<EntryProto> = from_items(items)?;
                return Ok(entries);
            }
            None => {
                return Err(anyhow::Error::msg("Error listing DynamoDB active EntryProto").into());
            }
        }
    }
}
