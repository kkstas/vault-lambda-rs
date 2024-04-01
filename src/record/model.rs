use aws_sdk_dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use super::TABLE_NAME;
use crate::{utils::time::get_today_datetime, AResult};

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub pk: String,
    pub sk: String,
    pub name: String,
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Deserialize)]
pub struct RecordFC {
    pub name: String,
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl From<RecordFC> for Record {
    fn from(record: RecordFC) -> Self {
        Record {
            pk: String::from("Record"),
            sk: get_today_datetime(),
            name: record.name,
            amount: record.amount,
            unit: record.unit,
        }
    }
}

impl Record {
    pub async fn ddb_create(client: Client, record_fc: RecordFC) -> AResult<()> {
        let item = to_item(Into::<Record>::into(record_fc))?;

        let req = client
            .put_item()
            .table_name(TABLE_NAME.to_owned())
            .set_item(Some(item));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_delete(client: Client, sk: String) -> AResult<()> {
        let req = client
            .delete_item()
            .table_name(TABLE_NAME.to_owned())
            .key("pk", AttributeValue::S(String::from("Record")))
            .key("sk", AttributeValue::S(sk));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_query_from_to(
        client: Client,
        from: String,
        to: String,
    ) -> AResult<Vec<Record>> {
        let res = client
            .execute_statement()
            .statement(format!(
                r#"SELECT * FROM "{}" WHERE "pk" = ? AND "sk" >= ? AND "sk" <= ?"#,
                TABLE_NAME
            ))
            .set_parameters(Some(vec![
                AttributeValue::S("Record".to_string()),
                AttributeValue::S(from),
                AttributeValue::S(to),
            ]))
            .send()
            .await?;

        match res.items {
            Some(items) => {
                let records: Vec<Record> = from_items(items)?;
                return Ok(records);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB Records. {:?}").into());
            }
        }
    }

    pub async fn ddb_query(client: Client, sk: String) -> AResult<Vec<Record>> {
        let query = client
            .query()
            .table_name(TABLE_NAME.to_owned())
            .key_condition_expression("pk = :pk AND sk >= :sk")
            .expression_attribute_values(":pk", AttributeValue::S(String::from("Record")))
            .expression_attribute_values(":sk", AttributeValue::S(sk));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let records: Vec<Record> = from_items(items)?;
                return Ok(records);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB Records. {:?}").into());
            }
        }
    }
}
