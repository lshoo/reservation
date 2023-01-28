use std::collections::VecDeque;

use crate::{
    get_user_resource_cond,
    pager::{Id, PageInfo, Paginator},
    Error, FilterPager, Normalize, ReservationFilter, ReservationFilterBuilder, ReservationStatus,
    ToSql, Validator,
};

impl ReservationFilterBuilder {
    pub fn build(&self) -> Result<ReservationFilter, Error> {
        let mut filter = self
            .private_build()
            .expect("failed to build ReservationFilter");

        filter.normalize()?;

        Ok(filter)
    }
}

impl Validator for ReservationFilter {
    fn validate(&self) -> Result<(), Error> {
        if self.page_size < 10 || self.page_size > 100 {
            return Err(Error::InvalidPageSize(self.page_size));
        }

        if let Some(cursor) = self.cursor {
            if cursor < 0 {
                return Err(Error::InvalidCursor(cursor));
            }
        }

        ReservationStatus::from_i32(self.status).ok_or(Error::InvalidStatus(self.status))?;

        Ok(())
    }
}

impl Normalize for ReservationFilter {
    fn do_normalize(&mut self) {
        println!("reservation filter normalizing");
        if self.status == ReservationStatus::Unknown as i32 {
            self.status = ReservationStatus::Pending as i32
        }
    }
}

impl ToSql for ReservationFilter {
    fn to_sql(&self) -> String {
        let middle_plus = i64::from(self.cursor.is_some());
        let limit = self.page_size + 1 + middle_plus;

        let status = self.get_status();

        let cursor_cond = if self.desc {
            format!("id <= {}", self.get_cursor())
        } else {
            format!("id >= {}", self.get_cursor())
        };

        let user_resource_cond = get_user_resource_cond(&self.user_id, &self.resource_id);

        let direction = if self.desc { "DESC" } else { "ASC" };

        format!("SELECT * FROM rsvp.reservations WHERE status = '{}'::rsvp.reservation_status AND {} AND {} ORDER BY id {} LIMIT {}",
        status, cursor_cond, user_resource_cond, direction, limit)
    }
}

impl ReservationFilter {
    pub fn get_status(&self) -> ReservationStatus {
        ReservationStatus::from_i32(self.status).unwrap()
    }

    pub fn get_cursor(&self) -> i64 {
        self.cursor.unwrap_or(if self.desc { i64::MAX } else { 0 })
    }

    pub fn get_pager<T: Id>(&self, data: &mut VecDeque<T>) -> FilterPager {
        let page_info = self.page_info();
        let pager = page_info.get_pager(data);

        pager.into()
    }

    pub fn next_page(&self, pager: &FilterPager) -> Option<Self> {
        let page_info = self.page_info();
        let pager = pager.into();
        let page_info = page_info.next_page(&pager);
        page_info.map(|pi| Self {
            user_id: self.user_id.clone(),
            resource_id: self.resource_id.clone(),
            status: self.status,
            cursor: pi.cursor,
            page_size: pi.page_size,
            desc: pi.desc,
        })
    }

    pub fn page_info(&self) -> PageInfo {
        PageInfo {
            cursor: self.cursor,
            page_size: self.page_size,
            desc: self.desc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pager::pager_test_utils::generate_test_ids, ReservationFilterBuilder};

    #[test]
    fn filter_generate_sql_should_correct() {
        let filter = ReservationFilterBuilder::default()
            .user_id("james id")
            .build()
            .unwrap();
        let sql = filter.to_sql();

        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 0 AND user_id = 'james id' ORDER BY id ASC LIMIT 11"
        );

        let filter = ReservationFilterBuilder::default()
            .user_id("james id")
            .resource_id("test")
            .build()
            .unwrap();
        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 0 AND user_id = 'james id' AND resource_id = 'test' ORDER BY id ASC LIMIT 11"
        );

        let filter = ReservationFilterBuilder::default()
            .desc(true)
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id <= 9223372036854775807 AND TRUE ORDER BY id DESC LIMIT 11"
        );

        let filter = ReservationFilterBuilder::default()
            .user_id("james id")
            .cursor(100)
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 100 AND user_id = 'james id' ORDER BY id ASC LIMIT 12"
        );

        let filter = ReservationFilterBuilder::default()
            .user_id("james id")
            .cursor(10)
            .desc(true)
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id <= 10 AND user_id = 'james id' ORDER BY id DESC LIMIT 12"
        );
    }

    #[test]
    fn filter_with_pager_should_generate_correct_sql() {
        let filter = ReservationFilterBuilder::default()
            .resource_id("test")
            .build()
            .unwrap();
        let mut items = generate_test_ids(1, 11);
        let pager = filter.get_pager(&mut items);
        assert_eq!(pager.prev, None);
        assert_eq!(pager.next, Some(10));

        let filter = filter.next_page(&pager).unwrap();
        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 10 AND resource_id = 'test' ORDER BY id ASC LIMIT 12"
        );
        let mut items = generate_test_ids(10, 20);
        let pager = filter.get_pager(&mut items);
        assert_eq!(pager.prev, Some(11));
        assert_eq!(pager.next, None);
    }
}
