use crate::AResult;
use aws_sdk_dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, from_items, to_item};

use super::TABLE_NAME;

#[derive(Serialize, Deserialize)]
pub struct EntryProto {
    pub pk: String, // "EntryProto::Active" || "EntryProto::Inactive"
    pub sk: String, // Primary key of referenced entry, e.g. "Entry::Dream"
    pub title: String,
}

#[derive(Clone, Deserialize)]
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
    pub async fn set_as_inactive(client: Client, sk: String) -> AResult<()> {
        if !(sk.starts_with("Entry::")) {
            return Err(anyhow::Error::msg("Invalid EntryProto sort key").into());
        }

        let active_query_res = match EntryProto::ddb_find(
            client.clone(),
            String::from("EntryProto::Active"),
            sk.clone(),
        )
        .await
        {
            Ok(res) => res,
            Err(_) => {
                return Err(anyhow::Error::msg(
                    "Active EntryProto with provided sort key does not exist",
                )
                .into())
            }
        };

        if EntryProto::ddb_find(
            client.clone(),
            String::from("EntryProto::Inactive"),
            sk.clone(),
        )
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
        client
            .put_item()
            .table_name(TABLE_NAME.to_owned())
            .set_item(Some(item))
            .send()
            .await?;

        EntryProto::ddb_delete(client, String::from("EntryProto::Active"), sk).await?;
        Ok(())
    }

    pub async fn set_as_active(client: Client, sk: String) -> AResult<()> {
        if !(sk.starts_with("Entry::")) {
            return Err(anyhow::Error::msg("Invalid EntryProto sort key").into());
        }

        if EntryProto::ddb_find(
            client.clone(),
            String::from("EntryProto::Active"),
            sk.clone(),
        )
        .await
        .is_ok()
        {
            return Err(
                anyhow::Error::msg("EntryProto already exists in DynamoDB as active").into(),
            );
        }

        let inactive_entry = match EntryProto::ddb_find(
            client.clone(),
            String::from("EntryProto::Inactive"),
            sk.clone(),
        )
        .await
        {
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
        client
            .put_item()
            .table_name(TABLE_NAME.to_owned())
            .set_item(Some(item))
            .send()
            .await?;

        EntryProto::ddb_delete(client, String::from("EntryProto::Inactive"), sk).await?;
        Ok(())
    }

    pub async fn ddb_put_item(client: Client, entry_proto_fc: EntryProtoFC) -> AResult<()> {
        if EntryProto::ddb_find(
            client.clone(),
            String::from("EntryProto::Inactive"),
            entry_proto_fc.sk.clone(),
        )
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
        client
            .put_item()
            .table_name(TABLE_NAME.to_owned())
            .set_item(Some(item))
            .send()
            .await?;
        Ok(())
    }

    pub async fn ddb_delete(client: Client, pk: String, sk: String) -> AResult<()> {
        if !pk.starts_with("EntryProto::") {
            return Err(anyhow::Error::msg("Invalid EntryProto primary key").into());
        }
        client
            .delete_item()
            .table_name(TABLE_NAME.to_owned())
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk))
            .send()
            .await?;
        Ok(())
    }

    pub async fn ddb_list_active(client: Client) -> AResult<Vec<EntryProto>> {
        let query = client
            .query()
            .table_name(TABLE_NAME.to_owned())
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

    pub async fn ddb_find(client: Client, pk: String, sk: String) -> AResult<EntryProto> {
        if (pk != "EntryProto::Active") && (pk != "EntryProto::Inactive") {
            return Err(
                anyhow::Error::msg("Invalid EntryProto query partition key argument").into(),
            );
        }

        if !sk.starts_with("Entry::") {
            return Err(anyhow::Error::msg("Invalid EntryProto query sort key argument").into());
        }
        let res = client
            .get_item()
            .table_name(TABLE_NAME.to_owned())
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk))
            .send()
            .await?;

        let item = res
            .item
            .ok_or(anyhow::Error::msg("Error querying DynamoDB entries"))?;

        Ok(from_item(item)?)
    }

    pub async fn ddb_list_inactive(client: Client) -> AResult<Vec<EntryProto>> {
        let query = client
            .query()
            .table_name(TABLE_NAME.to_owned())
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
