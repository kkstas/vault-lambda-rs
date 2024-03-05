use aws_sdk_dynamodb::{types::AttributeValue, Client};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use crate::AResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub pk: String,
    pub sk: String,
    pub duration_sec: u64,
    pub rating: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskFC {
    pub pk: String,
    pub duration_sec: u64,
    pub rating: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl Task {
    pub fn new(model_fc: TaskFC) -> Self {
        Self {
            pk: model_fc.pk,
            sk: Utc::now().to_string(),
            duration_sec: model_fc.duration_sec,
            comment: model_fc.comment,
            rating: model_fc.rating,
        }
    }
}

// DynamoDB handlers
impl Task {
    pub async fn ddb_create(client: Client, table_name: String, task_fc: TaskFC) -> AResult<()> {
        let model = Task::new(task_fc);
        let item = to_item(model.clone())?;

        let req = client
            .put_item()
            .table_name(table_name)
            .set_item(Some(item));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_delete(
        client: Client,
        table_name: String,
        pk: String,
        sk: String,
    ) -> AResult<()> {
        let req = client
            .delete_item()
            .table_name(table_name)
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_query(
        client: Client,
        table_name: String,
        pk: String,
        sk: String,
    ) -> AResult<Vec<Task>> {
        let query = client
            .query()
            .table_name(table_name)
            .key_condition_expression("pk = :pk AND sk >= :sk")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk", AttributeValue::S(sk));

        let res = query.send().await?;
        match res.items {
            Some(items) => {
                let tasks: Vec<Task> = from_items(items)?;
                return Ok(tasks);
            }
            None => {
                return Err(anyhow::Error::msg("Error querying DynamoDB tasks. {:?}").into());
            }
        }
    }
}

// ze starego proj
// #[derive(Debug, Serialize, Deserialize)]
// pub struct TaskListEntry {
//     pub pk: String,            // "TaskList::Inactive" || "TaskList::Active"
//     pub sk: String,            // Primary key of referenced task, e.g. "Task::Workout"
//     pub readable_name: String, // Readable name of referenced task, e.g. "Workout"
//     pub has_description: bool,
//     pub has_streak: bool,
//     pub has_reps: bool,
//     pub daily_reps_minimum: Option<u8>,
//     pub weekly_streak_tolerance: Option<u8>,
//     pub is_timed: bool,
// }

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct Task {
//     pub pk: String,            // e.g. "Task::Workout"
//     pub sk: String,            // creation date in ISO 8601 format, e.g. "2021-08-01T00:00:00Z"
//     pub readable_name: String, // e.g. "Workout"
//
//     pub has_description: bool, // e.g. true if we want to describe daily how the task was executed
//     pub description: Option<String>, // e.g. "Ran 5 miles, did 50 pushups, and 50 situps"
//
//     pub has_streak: bool,    // e.g. true if we want to track the daily streak
//     pub streak: Option<u32>, // e.g. 5 if we have a 5-day streak
//
//     pub has_reps: bool, // e.g. true if we want to add multiple tasks per day
//     pub daily_reps_minimum: Option<u8>, // e.g. 2 if we want to do create at least 2 reps per day to keep the streak alive
//     pub weekly_streak_tolerance: Option<u8>, // e.g. 2 if we allow 2 days of inactivity before breaking the streak
//
//     pub is_timed: bool, // e.g. true if we want to track the time spent on the task
//     pub total_time: Option<String>, // e.g. "00:30:00" if we want to track 30 minutes spent on the task
// }
