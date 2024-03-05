use aws_sdk_dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use crate::AResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskListEntryFC {
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
    pub fn new(t_fc: TaskListEntryFC, pk: String) -> Self {
        Self {
            pk,
            sk: t_fc.sk,
            readable_name: t_fc.readable_name,
            has_description: t_fc.has_description,
            has_streak: t_fc.has_streak,
            has_reps: t_fc.has_reps,
            daily_reps_minimum: t_fc.daily_reps_minimum,
            weekly_streak_tolerance: t_fc.weekly_streak_tolerance,
            is_timed: t_fc.is_timed,
        }
    }
}

impl TaskListEntry {
    pub async fn create(
        client: Client,
        table_name: String,
        task_list_entry_fc: TaskListEntryFC,
    ) -> AResult<()> {
        let active_query_res = TaskListEntry::find(
            client.clone(),
            table_name.clone(),
            String::from("TaskList::Active"),
            task_list_entry_fc.sk.clone(),
        )
        .await?;
        let inactive_query_res = TaskListEntry::find(
            client.clone(),
            table_name.clone(),
            String::from("TaskList::Inactive"),
            task_list_entry_fc.sk.clone(),
        )
        .await?;
        if active_query_res.is_empty() && inactive_query_res.is_empty() {
            TaskListEntry::ddb_put_item(
                client,
                table_name,
                TaskListEntry::new(task_list_entry_fc, "TaskList::Active".to_string()),
            )
            .await?;
            return Ok(());
        }
        return Err(anyhow::Error::msg("TaskListEntry with given sort key already exists").into());
    }

    pub async fn update(
        client: Client,
        table_name: String,
        task_list_entry_fu: TaskListEntryFC,
    ) -> AResult<()> {
        let active_query_res = TaskListEntry::find(
            client.clone(),
            table_name.clone(),
            String::from("TaskList::Active"),
            task_list_entry_fu.sk.clone(),
        )
        .await?;
        let inactive_query_res = TaskListEntry::find(
            client.clone(),
            table_name.clone(),
            String::from("TaskList::Inactive"),
            task_list_entry_fu.sk.clone(),
        )
        .await?;

        let active_empty = active_query_res.is_empty();
        let inactive_empty = inactive_query_res.is_empty();

        if active_empty && inactive_empty {
            return Err(
                anyhow::Error::msg("TaskListEntry with given sort key does not exist").into(),
            );
        }
        if !active_empty && !inactive_empty {
            return Err(anyhow::Error::msg("Corrupted data - TaskListEntry with given sort key exists in both active and inactive lists").into());
        }

        if active_query_res.len() > 1 {
            return Err(anyhow::Error::msg(
                "Corrupted data - there is more than one active TaskListEntry with that sort key",
            )
            .into());
        }

        if inactive_query_res.len() > 1 {
            return Err(anyhow::Error::msg(
                "Corrupted data - there is more than one inactive TaskListEntry with that sort key",
            )
            .into());
        }
        let task_list_entry_state = if !active_empty {
            String::from("TaskList::Active")
        } else {
            String::from("TaskList::Inactive")
        };

        let task_list_entry = TaskListEntry::new(task_list_entry_fu, task_list_entry_state);
        TaskListEntry::ddb_put_item(client, table_name, task_list_entry).await?;
        return Ok(());
    }

    pub async fn find(
        client: Client,
        table_name: String,
        pk: String,
        sk: String,
    ) -> AResult<Vec<TaskListEntry>> {
        TaskListEntry::ddb_find(client, table_name, pk, sk).await
    }
    pub async fn list_active(client: Client, table_name: String) -> AResult<Vec<TaskListEntry>> {
        TaskListEntry::ddb_list_active(client, table_name).await
    }
    pub async fn list_inactive(client: Client, table_name: String) -> AResult<Vec<TaskListEntry>> {
        TaskListEntry::ddb_list_inactive(client, table_name).await
    }
}

// Functions for direct interaction with DynamoDB
impl TaskListEntry {
    async fn ddb_find(
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

    async fn ddb_put_item(
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

    pub async fn ddb_list_inactive(
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
