use aws_sdk_dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use crate::AResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListEntry {
    pub pk: String,            // "TaskList::Inactive" || "TaskList::Active"
    pub sk: String,            // Primary key of referenced task, e.g. "Task::Workout"
    pub readable_name: String, // Readable name of referenced task, e.g. "Workout"
    pub has_description: bool,
    pub has_streak: bool,
    pub has_reps: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_reps_minimum: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly_streak_tolerance: Option<u8>,
    pub is_timed: bool,
}

impl TaskListEntry {
    pub async fn ddb_create(
        client: Client,
        table_name: String,
        task_list_entry: TaskListEntry,
    ) -> AResult<()> {
        if (task_list_entry.pk != "TaskList::Active")
            && (task_list_entry.pk != "TaskList::Inactive")
        {
            return Err(anyhow::Error::msg("Invalid TaskListEntry partition key. {:?}").into());
        }
        if !task_list_entry.sk.starts_with("Task::") {
            return Err(anyhow::Error::msg("Invalid TaskListEntry sort key. {:?}").into());
        }

        let item = to_item(task_list_entry)?;

        let req = client
            .put_item()
            .table_name(table_name)
            .set_item(Some(item));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_move_to_inactive(// client: Client,
        // table_name: String,
        // pk: String,
        // sk: String,
    ) -> AResult<()> {
        // // TODO: if pk is "TaskList::Inactive" or "TaskList::Active" allow deletion
        // let req = client
        //     .delete_item()
        //     .table_name(table_name)
        //     .key("pk", AttributeValue::S(pk))
        //     .key("sk", AttributeValue::S(sk));
        //
        // req.send().await?;
        // Ok(())
        todo!()
    }

    pub async fn ddb_query(
        client: Client,
        table_name: String,
        pk: String,
        sk: String,
    ) -> AResult<Vec<TaskListEntry>> {
        if (pk != "TaskList::Active") && (pk != "TaskList::Inactive") {
            return Err(anyhow::Error::msg(
                "Invalid TaskListEntry query partition key argument. {:?}",
            )
            .into());
        }

        if !sk.starts_with("Task::") {
            return Err(
                anyhow::Error::msg("Invalid TaskListEntry query sort key argument. {:?}").into(),
            );
        }

        let query = client
            .query()
            .table_name(table_name)
            .key_condition_expression("pk = :pk AND sk = :sk")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk", AttributeValue::S(sk));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let tasks: Vec<TaskListEntry> = from_items(items)?;
                return Ok(tasks);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB tasks. {:?}").into());
            }
        }
    }
    pub async fn ddb_list_active(
        client: Client,
        table_name: String,
    ) -> AResult<Vec<TaskListEntry>> {
        let query = client
            .query()
            .table_name(table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(":pk", AttributeValue::S("TaskList::Active".to_string()));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let tasks: Vec<TaskListEntry> = from_items(items)?;
                return Ok(tasks);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB tasks. {:?}").into());
            }
        }
    }
    pub async fn ddb_list_all_inactive(
        client: Client,
        table_name: String,
    ) -> AResult<Vec<TaskListEntry>> {
        let query = client
            .query()
            .table_name(table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S("TaskList::Inactive".to_string()),
            );

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let tasks: Vec<TaskListEntry> = from_items(items)?;
                return Ok(tasks);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB tasks. {:?}").into());
            }
        }
    }
}
