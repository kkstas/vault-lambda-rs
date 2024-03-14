use aws_sdk_dynamodb::{types::AttributeValue, Client};
use chrono::{Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use crate::{web::taskproto::model::TaskProto, AResult};

fn get_today_datetime() -> String {
    let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
    Utc::now()
        .with_timezone(&tz_offset)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn get_date_x_days_ago(x: i64) -> String {
    let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
    (Utc::now().with_timezone(&tz_offset) + Duration::days(-x))
        .format("%Y-%m-%d")
        .to_string()
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub pk: String,            // e.g. "Task::Workout"
    pub sk: String,            // creation date in ISO 8601 format, e.g. "2021-08-01T00:00:00Z"
    pub readable_name: String, // e.g. "Workout"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // e.g. "Ran 5 miles, did 50 pushups, and 50 situps"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub streak: Option<u32>, // e.g. 5 if we have a 5-day streak

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rep_number: Option<u8>, // e.g. 1 if first rep of the day

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>, // e.g. "00:30:00" if we want to track 30 minutes spent on the task
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskFC {
    pub pk: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // e.g. "Ran 5 miles, did 50 pushups, and 50 situps"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>, // e.g. "00:30:00" if we want to track 30 minutes spent on the task
}

// DynamoDB handlers
impl Task {
    pub async fn ddb_create(client: Client, table_name: String, task_fc: TaskFC) -> AResult<()> {
        let mut task_to_create: Task = Task::default();

        let task_proto_result = TaskProto::ddb_find(
            client.clone(),
            table_name.clone(),
            "TaskProto::Active".to_string(),
            task_fc.pk.clone(),
        )
        .await?;
        if task_proto_result.is_empty() {
            return Err(anyhow::Error::msg(format!(
                "TaskProto for given task {} not found",
                task_fc.pk
            ))
            .into());
        }
        let task_proto = task_proto_result.first().unwrap();
        task_to_create.pk = task_proto.sk.clone();
        task_to_create.sk = get_today_datetime();
        task_to_create.readable_name = task_proto.readable_name.clone();

        if task_proto.has_description {
            if let Some(description) = &task_fc.description {
                task_to_create.description = Some(description.to_string());
            }
        }

        if task_proto.is_timed {
            if let Some(total_time) = &task_fc.total_time {
                task_to_create.total_time = Some(total_time.to_string());
            }
        }

        if task_proto.has_streak == true {
            let last_week_tasks = Task::last_7_days_of_given_task(
                client.clone(),
                table_name.to_string(),
                task_to_create.pk.clone(),
            )
            .await?;

            if task_proto.has_reps == true {
                task_to_create.streak = Some(0);
            } else {
                // If task is not repeatable, don't let it be created if one already exists for today
                for task in last_week_tasks.iter() {
                    if task.sk.starts_with(&get_date_x_days_ago(0)) {
                        return Err(anyhow::Error::msg("Task for today already exists").into());
                    }
                }

                task_to_create.streak = Some(Task::compute_non_reps_streak(
                    task_proto.weekly_streak_tolerance.unwrap(),
                    Task::last_7_days_of_given_task(
                        client.clone(),
                        table_name.to_string(),
                        task_to_create.pk.clone(),
                    )
                    .await?,
                )?);
            }
        }

        let item = to_item(task_to_create)?;

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
        if !pk.starts_with("Task::") {
            return Err(anyhow::Error::msg("Invalid Task sort key").into());
        }
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

// helper functions
impl Task {
    fn compute_non_reps_streak(
        weekly_streak_tolerance: u8,
        last_week_tasks: Vec<Task>,
    ) -> AResult<u32> {
        let mut streak: u32 = 1; // If there are no tasks for the last 7 days, the streak starts at 1

        for day in 0..weekly_streak_tolerance {
            let date = get_date_x_days_ago(day as i64);

            if let Some(t) = last_week_tasks
                .iter()
                .find(|task| task.sk.starts_with(&date))
            {
                streak = t.streak.unwrap() + 1;
                break;
            }
        }

        // If streak is starting out, it shouldn't be reversed to one
        let min_len = 7 - weekly_streak_tolerance;
        if streak > min_len.into() && (last_week_tasks.len() as u8) < min_len {
            return Ok(1);
        }

        Ok(streak)
    }

    async fn last_7_days_of_given_task(
        client: Client,
        table_name: String,
        pk: String,
    ) -> AResult<Vec<Task>> {
        let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
        let week_ago = (Utc::now().with_timezone(&tz_offset) + Duration::days(-7))
            .format("%Y-%m-%d")
            .to_string();

        let tasks: Vec<Task> =
            Task::ddb_query(client.clone(), table_name.to_string(), pk, week_ago.clone()).await?;
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_non_reps_streak() {
        let mut t1 = Task::default();
        t1.streak = Some(6);
        t1.sk = get_date_x_days_ago(2);

        let mut t2 = Task::default();
        t2.streak = Some(5);
        t2.sk = get_date_x_days_ago(3);

        let mut t3 = Task::default();
        t3.streak = Some(4);
        t3.sk = get_date_x_days_ago(4);

        let v = vec![t1, t2, t3];
        println!("{:#?}", v);

        assert_eq!(Task::compute_non_reps_streak(4, v.clone()).unwrap(), 7);
        assert_eq!(Task::compute_non_reps_streak(3, v.clone()).unwrap(), 1);
        assert_eq!(Task::compute_non_reps_streak(1, v).unwrap(), 1);
    }
}
