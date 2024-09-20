use chrono::{DateTime, Utc};

pub fn display_datetime(datetime: DateTime<Utc>) -> String {
    datetime.with_timezone(&chrono::Local).to_rfc3339()
}
