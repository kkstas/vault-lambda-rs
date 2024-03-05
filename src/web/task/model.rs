use aws_sdk_dynamodb::{types::AttributeValue, Client};
use chrono::{FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use crate::AResult;

fn get_today_datetime() -> String {
    let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
    Utc::now()
        .with_timezone(&tz_offset)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub pk: String,            // e.g. "Task::Workout"
    pub sk: String,            // creation date in ISO 8601 format, e.g. "2021-08-01T00:00:00Z"
    pub readable_name: String, // e.g. "Workout"

    pub has_description: bool, // e.g. true if we want to describe daily how the task was executed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // e.g. "Ran 5 miles, did 50 pushups, and 50 situps"

    pub has_streak: bool, // e.g. true if we want to track the daily streak
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streak: Option<u32>, // e.g. 5 if we have a 5-day streak

    pub has_reps: bool, // e.g. true if we want to add multiple tasks per day
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_reps_minimum: Option<u8>, // e.g. 2 if we want to do create at least 2 reps per day to keep the streak alive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly_streak_tolerance: Option<u8>, // e.g. 2 if we allow 2 days of inactivity before breaking the streak

    pub is_timed: bool, // e.g. true if we want to track the time spent on the task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>, // e.g. "00:30:00" if we want to track 30 minutes spent on the task
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskFC {
    pub pk: String,
    pub readable_name: String, // e.g. "Workout"

    pub has_description: bool, // e.g. true if we want to describe daily how the task was executed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // e.g. "Ran 5 miles, did 50 pushups, and 50 situps"

    pub has_streak: bool, // e.g. true if we want to track the daily streak
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streak: Option<u32>, // e.g. 5 if we have a 5-day streak

    pub has_reps: bool, // e.g. true if we want to add multiple tasks per day
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_reps_minimum: Option<u8>, // e.g. 2 if we want to do create at least 2 reps per day to keep the streak alive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly_streak_tolerance: Option<u8>, // e.g. 2 if we allow 2 days of inactivity before breaking the streak

    pub is_timed: bool, // e.g. true if we want to track the time spent on the task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>, // e.g. "00:30:00" if we want to track 30 minutes spent on the task
}

impl Task {
    pub fn new(model_fc: TaskFC) -> Self {
        Self {
            pk: model_fc.pk,
            sk: get_today_datetime(),
            readable_name: model_fc.readable_name,
            has_description: model_fc.has_description,
            description: model_fc.description,
            has_streak: model_fc.has_streak,
            streak: model_fc.streak,
            has_reps: model_fc.has_reps,
            daily_reps_minimum: model_fc.daily_reps_minimum,
            weekly_streak_tolerance: model_fc.weekly_streak_tolerance,
            is_timed: model_fc.is_timed,
            total_time: model_fc.total_time,
        }
    }
}

// DynamoDB handlers
impl Task {
    pub async fn ddb_create(client: Client, table_name: String, task_fc: TaskFC) -> AResult<()> {
        let model = Task::new(task_fc);
        let item = to_item(model)?;

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
        // TODO: if pk starts with "Task::" allow deletion
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
