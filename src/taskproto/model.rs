use aws_sdk_dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use crate::AResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskProto {
    pub pk: String,            // "TaskProto::Inactive" || "TaskProto::Active"
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
pub struct TaskProtoFC {
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

impl TaskProto {
    pub fn new(t_fc: TaskProtoFC, pk: String) -> Self {
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

impl TaskProto {
    pub async fn set_as_active(client: Client, table_name: String, sk: String) -> AResult<()> {
        let found_inactive_task_arr = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Inactive"),
            sk.clone(),
        )
        .await?;
        if found_inactive_task_arr.is_empty() {
            return Err(anyhow::Error::msg(
                "Inactive TaskProto with given sort key does not exist",
            )
            .into());
        }
        let mut found_inactive_task: TaskProto = found_inactive_task_arr.first().unwrap().clone();

        let found_active_task_arr = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Active"),
            sk.clone(),
        )
        .await?;

        if !found_active_task_arr.is_empty() {
            return Err(
                anyhow::Error::msg("Active TaskProto with given sort key already exists").into(),
            );
        }

        found_inactive_task.pk = String::from("TaskProto::Active");
        TaskProto::ddb_put_item(client.clone(), table_name.clone(), found_inactive_task).await?;
        TaskProto::ddb_delete(client, table_name, String::from("TaskProto::Inactive"), sk).await?;
        Ok(())
    }

    pub async fn set_as_inactive(client: Client, table_name: String, sk: String) -> AResult<()> {
        let found_task_arr = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Active"),
            sk.clone(),
        )
        .await?;
        if found_task_arr.is_empty() {
            return Err(
                anyhow::Error::msg("Active TaskProto with given sort key does not exist").into(),
            );
        }
        let mut found_task: TaskProto = found_task_arr.first().unwrap().clone();

        let found_inactive_task_arr = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Inactive"),
            sk.clone(),
        )
        .await?;

        if !found_inactive_task_arr.is_empty() {
            return Err(anyhow::Error::msg(
                "Inactive TaskProto with given sort key already exists",
            )
            .into());
        }

        found_task.pk = String::from("TaskProto::Inactive");
        TaskProto::ddb_put_item(client.clone(), table_name.clone(), found_task).await?;
        TaskProto::ddb_delete(client, table_name, String::from("TaskProto::Active"), sk).await?;
        Ok(())
    }

    pub async fn create(
        client: Client,
        table_name: String,
        task_list_entry_fc: TaskProtoFC,
    ) -> AResult<()> {
        let active_query_res = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Active"),
            task_list_entry_fc.sk.clone(),
        )
        .await?;
        let inactive_query_res = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Inactive"),
            task_list_entry_fc.sk.clone(),
        )
        .await?;
        if active_query_res.is_empty() && inactive_query_res.is_empty() {
            if !task_list_entry_fc.has_streak && task_list_entry_fc.has_reps {
                return Err(anyhow::Error::msg(
                    "TaskProto with reps must have 'has_streak' property set to true",
                )
                .into());
            }

            if task_list_entry_fc.has_streak && task_list_entry_fc.weekly_streak_tolerance.is_none()
            {
                return Err(anyhow::Error::msg(
                    "TaskProto with streaks must have weekly_streak_tolerance",
                )
                .into());
            }
            if !task_list_entry_fc.has_streak
                && task_list_entry_fc.weekly_streak_tolerance.is_some()
            {
                return Err(anyhow::Error::msg(
                    "TaskProto without streaks must not have weekly_streak_tolerance",
                )
                .into());
            }

            if task_list_entry_fc.has_reps && task_list_entry_fc.daily_reps_minimum.is_none() {
                return Err(
                    anyhow::Error::msg("TaskProto with reps must have daily_reps_minimum").into(),
                );
            }

            if !task_list_entry_fc.has_reps && task_list_entry_fc.daily_reps_minimum.is_some() {
                return Err(anyhow::Error::msg(
                    "TaskProto without reps must not have daily_reps_minimum",
                )
                .into());
            }

            TaskProto::ddb_put_item(
                client,
                table_name,
                TaskProto::new(task_list_entry_fc, "TaskProto::Active".to_string()),
            )
            .await?;
            return Ok(());
        }
        return Err(anyhow::Error::msg("TaskProto with given sort key already exists").into());
    }

    pub async fn update(
        client: Client,
        table_name: String,
        task_list_entry_fu: TaskProtoFC,
    ) -> AResult<()> {
        let active_query_res = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Active"),
            task_list_entry_fu.sk.clone(),
        )
        .await?;
        let inactive_query_res = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            String::from("TaskProto::Inactive"),
            task_list_entry_fu.sk.clone(),
        )
        .await?;

        let active_empty = active_query_res.is_empty();
        let inactive_empty = inactive_query_res.is_empty();

        if active_empty && inactive_empty {
            return Err(anyhow::Error::msg("TaskProto with given sort key does not exist").into());
        }
        if !active_empty && !inactive_empty {
            return Err(anyhow::Error::msg("Corrupted data - TaskProto with given sort key exists in both active and inactive lists").into());
        }

        if active_query_res.len() > 1 {
            return Err(anyhow::Error::msg(
                "Corrupted data - there is more than one active TaskProto with that sort key",
            )
            .into());
        }

        if inactive_query_res.len() > 1 {
            return Err(anyhow::Error::msg(
                "Corrupted data - there is more than one inactive TaskProto with that sort key",
            )
            .into());
        }
        let task_list_entry_state = if !active_empty {
            String::from("TaskProto::Active")
        } else {
            String::from("TaskProto::Inactive")
        };

        let task_list_entry = TaskProto::new(task_list_entry_fu, task_list_entry_state);
        TaskProto::ddb_put_item(client, table_name, task_list_entry).await?;
        return Ok(());
    }
}

// Functions for direct interaction with DynamoDB
impl TaskProto {
    pub async fn ddb_find(
        client: Client,
        table_name: String,
        pk: String,
        sk: String,
    ) -> AResult<Vec<TaskProto>> {
        if (pk != "TaskProto::Active") && (pk != "TaskProto::Inactive") {
            return Err(
                anyhow::Error::msg("Invalid TaskProto query partition key argument. {:?}").into(),
            );
        }

        if !sk.starts_with("Task::") {
            return Err(
                anyhow::Error::msg("Invalid TaskProto query sort key argument. {:?}").into(),
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
                let tasks: Vec<TaskProto> = from_items(items)?;
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
        task_list_entry: TaskProto,
    ) -> AResult<()> {
        if (task_list_entry.pk != "TaskProto::Active")
            && (task_list_entry.pk != "TaskProto::Inactive")
        {
            return Err(anyhow::Error::msg("Invalid TaskProto partition key. {:?}").into());
        }
        if !task_list_entry.sk.starts_with("Task::") {
            return Err(anyhow::Error::msg("Invalid TaskProto sort key. {:?}").into());
        }

        let item = to_item(task_list_entry)?;

        let req = client
            .put_item()
            .table_name(table_name)
            .set_item(Some(item));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_list_active(client: Client, table_name: String) -> AResult<Vec<TaskProto>> {
        let query = client
            .query()
            .table_name(table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(":pk", AttributeValue::S("TaskProto::Active".to_string()));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let tasks: Vec<TaskProto> = from_items(items)?;
                return Ok(tasks);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB tasks. {:?}").into());
            }
        }
    }

    pub async fn ddb_list_inactive(client: Client, table_name: String) -> AResult<Vec<TaskProto>> {
        let query = client
            .query()
            .table_name(table_name)
            .key_condition_expression("pk = :pk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S("TaskProto::Inactive".to_string()),
            );

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let tasks: Vec<TaskProto> = from_items(items)?;
                return Ok(tasks);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB tasks. {:?}").into());
            }
        }
    }

    async fn ddb_delete(client: Client, table_name: String, pk: String, sk: String) -> AResult<()> {
        if !pk.starts_with("TaskProto::") {
            return Err(anyhow::Error::msg("Invalid TaskProto primary key").into());
        }
        if !sk.starts_with("Task::") {
            return Err(anyhow::Error::msg("Invalid TaskProto sort key").into());
        }
        let req = client
            .delete_item()
            .table_name(table_name)
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk));

        req.send().await?;
        Ok(())
    }
}
