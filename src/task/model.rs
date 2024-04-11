use aws_sdk_dynamodb::{types::AttributeValue, Client};
use chrono::{Duration, Utc};
use chrono_tz::Europe;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_items, to_item};

use super::TABLE_NAME;
use crate::utils::time::{get_date_x_days_ago, get_today_datetime};
use crate::{taskproto::TaskProto, AResult};

#[derive(Default, Serialize, Deserialize, Clone)]
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

#[derive(Deserialize)]
pub struct TaskFC {
    pub pk: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>, // e.g. "Ran 5 miles, did 50 pushups, and 50 situps"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>, // e.g. "00:30:00" if we want to track 30 minutes spent on the task
}

// DynamoDB handlers
impl Task {
    pub async fn ddb_create(client: Client, task_fc: TaskFC) -> AResult<()> {
        let mut task_to_create: Task = Task::default();

        let task_proto = match TaskProto::ddb_find(
            client.clone(),
            "TaskProto::Active".to_string(),
            task_fc.pk.clone(),
        )
        .await
        {
            Ok(res) => res,
            Err(_) => {
                return Err(anyhow::Error::msg(format!(
                    "TaskProto for given task {} not found",
                    task_fc.pk
                ))
                .into());
            }
        };

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
            let last_week_tasks =
                Task::last_7_days_of_given_task(client.clone(), task_to_create.pk.clone()).await?;

            if task_proto.has_reps == true {
                let current_rep_data = Task::compute_reps_streak(
                    task_proto.daily_reps_minimum.unwrap(),
                    task_proto.weekly_streak_tolerance.unwrap(),
                    last_week_tasks.clone(),
                )?;
                task_to_create.streak = Some(current_rep_data.streak);
                task_to_create.rep_number = Some(current_rep_data.rep_number);
            } else {
                // If task is not repeatable, don't let it be created if one already exists for today
                for task in last_week_tasks.iter() {
                    if task.sk.starts_with(&get_date_x_days_ago(0)) {
                        return Err(anyhow::Error::msg("Task for today already exists").into());
                    }
                }

                task_to_create.streak = Some(Task::compute_non_reps_streak(
                    task_proto.weekly_streak_tolerance.unwrap(),
                    Task::last_7_days_of_given_task(client.clone(), task_to_create.pk.clone())
                        .await?,
                )?);
            }
        }

        let item = to_item(task_to_create)?;

        let req = client
            .put_item()
            .table_name(TABLE_NAME.to_owned())
            .set_item(Some(item));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_delete(client: Client, pk: String, sk: String) -> AResult<()> {
        if !pk.starts_with("Task::") {
            return Err(anyhow::Error::msg("Invalid Task primary key").into());
        }
        let req = client
            .delete_item()
            .table_name(TABLE_NAME.to_owned())
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk));

        req.send().await?;
        Ok(())
    }

    pub async fn ddb_query(client: Client, pk: String, sk: String) -> AResult<Vec<Task>> {
        let query = client
            .query()
            .table_name(TABLE_NAME.to_owned())
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

struct RepTaskDaySummary {
    date: String,
    streak: u32,
}

struct CurrentRepData {
    streak: u32,
    rep_number: u8,
}

// helper functions
impl Task {
    fn get_streaks_with_reps_week_summary(
        daily_rep_minimum: u8,
        last_week_tasks: Vec<Task>,
    ) -> Vec<RepTaskDaySummary> {
        let mut summary: Vec<RepTaskDaySummary> = Vec::new();

        for day in 1..7 {
            let date = get_date_x_days_ago(day as i64);

            let tasks_that_day: Vec<Task> = last_week_tasks
                .iter()
                .filter(|task| task.sk.starts_with(&date))
                .cloned()
                .collect();

            if tasks_that_day.len() >= daily_rep_minimum as usize {
                let mut streak_that_day = 0;
                // find streak that day
                for task in tasks_that_day.iter() {
                    if task.streak.unwrap() >= streak_that_day {
                        streak_that_day = task.streak.unwrap();
                    }
                }

                summary.push(RepTaskDaySummary {
                    date: date.clone(),
                    streak: streak_that_day,
                });
            }
        }
        summary
    }

    fn compute_reps_streak(
        daily_rep_minimum: u8,
        weekly_streak_tolerance: u8,
        last_week_tasks: Vec<Task>,
    ) -> AResult<CurrentRepData> {
        let mut last_found_streak: u32 = 0; // streak starts at 0 if there are no tasks for the last 7 days & today's tasks are less than daily_rep_minimum
        let mut today_streak_point: u32 = 0;

        let summary: Vec<RepTaskDaySummary> =
            Task::get_streaks_with_reps_week_summary(daily_rep_minimum, last_week_tasks.clone());

        for day in 0..weekly_streak_tolerance {
            let date = get_date_x_days_ago(day as i64);

            if let Some(t) = summary.iter().find(|task| task.date == date) {
                last_found_streak = t.streak;
                break;
            }
        }

        let today_tasks: Vec<Task> = last_week_tasks
            .iter()
            .filter(|task| task.sk.starts_with(&get_date_x_days_ago(0)))
            .cloned()
            .collect();

        let todays_rep_count_enough_for_streak =
            today_tasks.len() + 1 >= daily_rep_minimum as usize;

        if todays_rep_count_enough_for_streak {
            today_streak_point = 1;
        }

        let min_len = 7 - weekly_streak_tolerance;
        if last_found_streak > min_len.into() && ((summary.len() + 1) as u8) < min_len {
            return Ok(CurrentRepData {
                streak: last_found_streak + today_streak_point,
                rep_number: today_tasks.len() as u8 + 1,
            });
        }

        Ok(CurrentRepData {
            streak: last_found_streak + today_streak_point,
            rep_number: today_tasks.len() as u8 + 1,
        })
    }

    fn compute_non_reps_streak(
        weekly_streak_tolerance: u8,
        last_week_tasks: Vec<Task>,
    ) -> AResult<u32> {
        let mut streak: u32 = 1; // If there are no tasks for the last 7 days, the streak starts at 1

        for day in 1..=(weekly_streak_tolerance + 1) {
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

    async fn last_7_days_of_given_task(client: Client, pk: String) -> AResult<Vec<Task>> {
        let week_ago = (Utc::now().with_timezone(&Europe::Warsaw) + Duration::days(-7))
            .format("%Y-%m-%d")
            .to_string();

        let tasks: Vec<Task> = Task::ddb_query(client.clone(), pk, week_ago.clone()).await?;
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_reps_streak() {
        let mut t0 = Task::default();
        t0.streak = Some(6);
        t0.rep_number = Some(1);
        t0.sk = get_date_x_days_ago(0);

        let mut t1_1 = Task::default();
        t1_1.streak = Some(5);
        t1_1.rep_number = Some(1);
        t1_1.sk = get_date_x_days_ago(2);

        let mut t1_2 = Task::default();
        t1_2.streak = Some(6);
        t1_2.rep_number = Some(2);
        t1_2.sk = get_date_x_days_ago(2);

        let mut t1_3 = Task::default();
        t1_3.streak = Some(6);
        t1_3.rep_number = Some(3);
        t1_3.sk = get_date_x_days_ago(2);

        let mut t2_1 = Task::default();
        t2_1.streak = Some(4);
        t2_1.rep_number = Some(1);
        t2_1.sk = get_date_x_days_ago(3);

        let mut t2_2 = Task::default();
        t2_2.streak = Some(5);
        t2_2.rep_number = Some(2);
        t2_2.sk = get_date_x_days_ago(3);

        let mut t3_1 = Task::default();
        t3_1.streak = Some(3);
        t3_1.rep_number = Some(1);
        t3_1.sk = get_date_x_days_ago(4);

        let mut t3_2 = Task::default();
        t3_2.streak = Some(4);
        t3_2.rep_number = Some(2);
        t3_2.sk = get_date_x_days_ago(4);

        let v = vec![t0, t1_1, t1_2, t1_3, t2_1, t2_2, t3_1, t3_2];

        let result = Task::compute_reps_streak(2, 3, v).unwrap();
        assert_eq!(result.streak, 7);
        assert_eq!(result.rep_number, 2);
    }

    #[test]
    fn test_get_streaks_with_reps_week_summary() {
        let mut t1_1 = Task::default();
        t1_1.streak = Some(5);
        t1_1.rep_number = Some(1);
        t1_1.sk = get_date_x_days_ago(2);

        let mut t1_2 = Task::default();
        t1_2.streak = Some(6);
        t1_2.rep_number = Some(2);
        t1_2.sk = get_date_x_days_ago(2);

        let mut t1_3 = Task::default();
        t1_3.streak = Some(6);
        t1_3.rep_number = Some(3);
        t1_3.sk = get_date_x_days_ago(2);

        let mut t2_1 = Task::default();
        t2_1.streak = Some(4);
        t2_1.rep_number = Some(1);
        t2_1.sk = get_date_x_days_ago(3);

        let mut t2_2 = Task::default();
        t2_2.streak = Some(5);
        t2_2.rep_number = Some(2);
        t2_2.sk = get_date_x_days_ago(3);

        let mut t3_1 = Task::default();
        t3_1.streak = Some(4);
        t3_1.rep_number = Some(1);
        t3_1.sk = get_date_x_days_ago(4);

        let v = vec![t1_1, t1_2, t1_3, t2_1, t2_2, t3_1];

        let result = Task::get_streaks_with_reps_week_summary(2, v);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].streak, 6);
        assert_eq!(result[1].streak, 5);
    }

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

        assert_eq!(Task::compute_non_reps_streak(4, v.clone()).unwrap(), 7);
        assert_eq!(Task::compute_non_reps_streak(3, v.clone()).unwrap(), 1);
        assert_eq!(Task::compute_non_reps_streak(1, v).unwrap(), 1);

        let v2 = vec![
            Task {
                streak: Some(6),
                sk: get_date_x_days_ago(1),
                ..Default::default()
            },
            Task {
                streak: Some(5),
                sk: get_date_x_days_ago(2),
                ..Default::default()
            },
            Task {
                streak: Some(4),
                sk: get_date_x_days_ago(3),
                ..Default::default()
            },
            Task {
                streak: Some(3),
                sk: get_date_x_days_ago(4),
                ..Default::default()
            },
            Task {
                streak: Some(2),
                sk: get_date_x_days_ago(5),
                ..Default::default()
            },
            Task {
                streak: Some(1),
                sk: get_date_x_days_ago(6),
                ..Default::default()
            },
        ];

        assert_eq!(Task::compute_non_reps_streak(4, v2.clone()).unwrap(), 7);
        assert_eq!(Task::compute_non_reps_streak(3, v2.clone()).unwrap(), 7);
        assert_eq!(Task::compute_non_reps_streak(2, v2.clone()).unwrap(), 7);
        assert_eq!(Task::compute_non_reps_streak(1, v2.clone()).unwrap(), 7);
        assert_eq!(Task::compute_non_reps_streak(0, v2).unwrap(), 7);

        let v3 = vec![
            Task {
                streak: Some(3),
                sk: get_date_x_days_ago(3),
                ..Default::default()
            },
            Task {
                streak: Some(2),
                sk: get_date_x_days_ago(4),
                ..Default::default()
            },
            Task {
                streak: Some(1),
                sk: get_date_x_days_ago(5),
                ..Default::default()
            },
        ];

        assert_eq!(Task::compute_non_reps_streak(3, v3.clone()).unwrap(), 4);
        assert_eq!(Task::compute_non_reps_streak(2, v3.clone()).unwrap(), 4);
        assert_eq!(Task::compute_non_reps_streak(1, v3.clone()).unwrap(), 1);
        assert_eq!(Task::compute_non_reps_streak(0, v3).unwrap(), 1);
    }
}
