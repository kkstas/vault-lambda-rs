use chrono::{Duration, Utc};
use chrono_tz::Europe;

pub fn get_today_datetime() -> String {
    Utc::now()
        .with_timezone(&Europe::Warsaw)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn get_date_x_days_ago(x: i64) -> String {
    (Utc::now().with_timezone(&Europe::Warsaw) + Duration::days(-x))
        .format("%Y-%m-%d")
        .to_string()
}
