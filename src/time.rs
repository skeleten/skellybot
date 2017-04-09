use std;
use ::chrono;
use chrono::DateTime;

pub fn system_time_to_date_time(t: std::time::SystemTime) -> DateTime<chrono::offset::utc::UTC> {
    let (sec, nsec) = match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => { // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        },
    };
    use chrono::TimeZone;
    chrono::offset::utc::UTC.timestamp(sec, nsec)
}

pub fn format_time(t: std::time::SystemTime) -> String {
    let t = ::time::system_time_to_date_time(t);
    use chrono::{Datelike, Timelike};
    format!("{:04}-{:02}-{:02} {:02}:{:02} UTC",
            t.year(),
            t.month(),
            t.day(),
            t.hour(),
            t.minute())
}
