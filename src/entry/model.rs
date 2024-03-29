use aws_sdk_dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use super::TABLE_NAME;
use crate::entryproto::EntryProto;
use crate::utils::time::get_date_x_days_ago;
use crate::AResult;

#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub pk: String, // e.g. "Entry::Dream"
    pub sk: String, // creation date in ISO 8601 format, e.g. "2021-08-01T00:00:00Z"
    pub title: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct EntryFC {
    pub pk: String,
    pub content: String,
}

impl Entry {
    pub async fn ddb_query(client: Client, pk: String, sk: String) -> AResult<Vec<Entry>> {
        let query = client
            .query()
            .table_name(TABLE_NAME.to_owned())
            .key_condition_expression("pk = :pk AND sk >= :sk")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk", AttributeValue::S(sk));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let entries: Vec<Entry> = from_items(items)?;
                return Ok(entries);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB entries. {:?}").into());
            }
        }
    }

    pub async fn ddb_put_item(client: Client, entry_fc: EntryFC) -> AResult<()> {
        let entry_proto = match EntryProto::ddb_find(
            client.clone(),
            String::from("EntryProto::Active"),
            entry_fc.pk.clone(),
        )
        .await
        {
            Ok(res) => res,
            Err(_) => {
                return Err(anyhow::Error::msg(
                    "EntryProto for given Entry does not exist in DynamoDB",
                )
                .into())
            }
        };

        let entry = Entry {
            pk: entry_proto.sk.clone(),
            sk: get_date_x_days_ago(0),
            title: entry_proto.title.clone(),
            content: entry_fc.content,
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
        if !pk.starts_with("Entry::") {
            return Err(anyhow::Error::msg("Invalid Entry primary key").into());
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
}
