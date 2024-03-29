pub mod pager;
mod request;
mod reservation;
mod reservation_filter;
mod reservation_query;
mod reservation_status;

use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use sqlx::postgres::types::PgRange;
use std::ops::Bound;

use crate::{convert_to_utc_time, Error};

pub fn validate_range(start: Option<&Timestamp>, end: Option<&Timestamp>) -> Result<(), Error> {
    if start.is_none() || end.is_none() {
        return Err(Error::InvalidTime);
    }

    let start = start.as_ref().unwrap();
    let end = end.as_ref().unwrap();

    if start.seconds >= end.seconds {
        return Err(Error::InvalidTime);
    }

    Ok(())
}

pub fn get_timespan(start: Option<&Timestamp>, end: Option<&Timestamp>) -> PgRange<DateTime<Utc>> {
    let start = convert_to_utc_time(start.as_ref().unwrap());
    let end = convert_to_utc_time(end.as_ref().unwrap());

    PgRange {
        start: Bound::Included(start),
        end: Bound::Excluded(end),
    }
}

pub fn get_user_resource_cond(user_id: &str, resource_id: &str) -> String {
    match (user_id.is_empty(), resource_id.is_empty()) {
        (true, true) => "TRUE".into(),
        (true, false) => format!("resource_id = '{resource_id}'"),
        (false, true) => format!("user_id = '{user_id}'"),
        (false, false) => format!("user_id = '{user_id}' AND resource_id = '{resource_id}'"),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;

    use prost_types::Timestamp;

    use crate::{convert_to_utc_time, types::validate_range};

    use super::get_timespan;

    #[test]
    fn validate_range_should_work_valid_range() {
        let start = Timestamp {
            seconds: 1,
            nanos: 0,
        };

        let end = Timestamp {
            seconds: 2,
            nanos: 0,
        };

        assert!(validate_range(Some(&start), Some(&end)).is_ok());
    }

    #[test]
    fn validate_range_should_reject_invalid_range() {
        let start = Timestamp {
            seconds: 3,
            nanos: 0,
        };

        let end = Timestamp {
            seconds: 2,
            nanos: 0,
        };

        assert!(validate_range(Some(&start), Some(&end)).is_err());
    }

    #[test]
    fn get_timespan_should_work_for_valid_start_end() {
        let start = Timestamp {
            seconds: 1,
            nanos: 0,
        };

        let end = Timestamp {
            seconds: 2,
            nanos: 0,
        };

        let range = get_timespan(Some(&start), Some(&end));

        assert_eq!(range.start, Bound::Included(convert_to_utc_time(&start)));
        assert_eq!(range.end, Bound::Excluded(convert_to_utc_time(&end)));
    }
}
