use chrono::{Duration, FixedOffset, Utc};

pub fn get_today_datetime() -> String {
    let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
    Utc::now()
        .with_timezone(&tz_offset)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn get_date_x_days_ago(x: i64) -> String {
    let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
    (Utc::now().with_timezone(&tz_offset) + Duration::days(-x))
        .format("%Y-%m-%d")
        .to_string()
}
