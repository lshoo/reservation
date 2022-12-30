use abi::Validator;
use async_trait::async_trait;

use crate::{Error, ReservationId, ReservationManager, Rsvp};

use sqlx::{PgPool, Row};

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, Error> {
        rsvp.validate()?;

        let timespan = rsvp.get_timespan();

        let status = abi::ReservationStatus::from_i32(rsvp.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let id = sqlx::query(
            "INSERT INTO rsvp.reservations (user_id, resource_id, timespan, note, status) VALUES ($1, $2, $3, $4,
            $5::rsvp.reservation_status) RETURNING id"
        )
            .bind(rsvp.user_id.clone())
            .bind(rsvp.resource_id.clone())
            .bind(timespan)
            .bind(rsvp.note.clone())
            .bind(status.to_string())
            .fetch_one(&self.pool)
            .await?.get(0);

        rsvp.id = id;

        Ok(rsvp)
    }

    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        id.validate()?;
        let rsvp: abi::Reservation = sqlx::query_as(
            "UPDATE rsvp.reservations SET status = 'confirmed' where id = $1 and status = 'pending'
            RETURNING *",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(rsvp)
    }

    async fn update_note(
        &self,
        id: ReservationId,
        note: String,
    ) -> Result<abi::Reservation, Error> {
        id.validate()?;
        let rsvp: abi::Reservation =
            sqlx::query_as("UPDATE rsvp.reservations SET note = $1 where id = $2 RETURNING *")
                .bind(note)
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        Ok(rsvp)
    }

    async fn delete(&self, id: ReservationId) -> Result<(), Error> {
        id.validate()?;
        sqlx::query("DELETE FROM rsvp.reservations WHERE id = $1 ")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        id.validate()?;
        let rsvp: abi::Reservation =
            sqlx::query_as("SELECT * FROM rsvp.reservations WHERE id = $1 ")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        Ok(rsvp)
    }

    async fn query(&self, query: abi::ReservationQuery) -> Result<Vec<abi::Reservation>, Error> {
        let user_id = string_to_opt(&query.user_id);
        let resource_id = string_to_opt(&query.resource_id);
        let range = query.get_timespan();
        let status = abi::ReservationStatus::from_i32(query.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let rsvps = sqlx::query_as(
            "SELECT * FROM rsvp.query($1, $2, $3, $4::rsvp.reservation_status, $5, $6, $7)",
        )
        .bind(user_id)
        .bind(resource_id)
        .bind(range)
        .bind(status.to_string())
        .bind(query.page)
        .bind(query.desc)
        .bind(query.page_size)
        .fetch_all(&self.pool)
        .await?;

        Ok(rsvps)
    }

    async fn filter(
        &self,
        filter: abi::ReservationFilter,
    ) -> Result<(abi::FilterPager, Vec<abi::Reservation>), Error> {
        // filter reservation by user_id, resource_id, status, and order by id
        let user_id = string_to_opt(&filter.user_id);
        let resource_id = string_to_opt(&filter.resource_id);
        let status = abi::ReservationStatus::from_i32(filter.status)
            .unwrap_or(abi::ReservationStatus::Pending);
        let page_size = if filter.page_size < 10 || filter.page_size > 100 {
            10
        } else {
            filter.page_size
        };

        let rsvps: Vec<abi::Reservation> = sqlx::query_as(
            "SELECT * FROM rsvp.filter($1, $2, $3::rsvp.reservation_status, $4, $5, $6)",
        )
        .bind(user_id)
        .bind(resource_id)
        .bind(status.to_string())
        .bind(filter.cursor)
        .bind(filter.desc)
        .bind(page_size)
        .fetch_all(&self.pool)
        .await?;

        // if the first id is current cursor, then we have prev, we start from 1
        // if len - start > page_size, then we have next, we end at len - 1.
        let has_prev = !rsvps.is_empty() && rsvps[0].id == filter.cursor;
        let start = usize::from(has_prev);

        let has_next = (rsvps.len() - start) as i32 > page_size;
        let end = if has_next {
            rsvps.len() - 1
        } else {
            rsvps.len()
        };

        let prev = if start == 1 { rsvps[start].id } else { -1 };
        let next = if end == rsvps.len() - 1 {
            rsvps[end].id
        } else {
            -1
        };

        let pager = abi::FilterPager {
            prev,
            next,
            // TODO how to get total efficiently?
            total: 0,
        };

        Ok((pager, rsvps))
    }
}

fn string_to_opt(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
#[cfg(test)]
mod tests {

    use abi::{ReservationFilterBuilder, ReservationQueryBuilder, ReservationStatus};
    use prost_types::Timestamp;

    use super::*;

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() -> Result<(), Error> {
        let (rsvp, _) = make_james_reservation(&migrated_pool).await;
        assert_ne!(rsvp.id, 0);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_conflict_reservation_should_rejected() -> Result<(), Error> {
        let (_rsvp1, manager) = make_james_reservation(&migrated_pool).await;

        let rsvp2 = abi::Reservation::new_pending(
            "alice id",
            "Ocean view room 5018",
            "2022-12-26T15:00:00-0700".parse().unwrap(),
            "2022-12-31T00:00:00-0700".parse().unwrap(),
            "I'll arrive at 3pm, Please help to upgrade to execuitive room if possible.",
        );

        // let _rsvp1 = manager.reserve(rsvp1).await.unwrap();
        let err = manager.reserve(rsvp2).await.unwrap_err();

        // println!("{:?}", err);

        if let abi::Error::ConflictReservation(_info) = err {
        } else {
            panic!("expected conflict reservation")
        }

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn change_status_should_work() -> Result<(), Error> {
        let (rsvp, manager) = make_alice_reservation(&migrated_pool).await;

        let rsvp = manager.change_status(rsvp.id).await?;

        assert_eq!(rsvp.status, ReservationStatus::Confirmed as i32);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn change_status_not_pending_should_do_nothing() -> Result<(), Error> {
        let (rsvp, manager) = make_alice_reservation(&migrated_pool).await;

        let rsvp = manager.change_status(rsvp.id).await?;

        assert_eq!(rsvp.status, ReservationStatus::Confirmed as i32);

        let ret = manager.change_status(rsvp.id).await.unwrap_err();

        assert_eq!(ret, Error::NotFound);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn update_note_should_work() -> Result<(), Error> {
        let (rsvp, manager) = make_alice_reservation(&migrated_pool).await;

        let rsvp = manager.update_note(rsvp.id, "007".into()).await?;

        assert_eq!(rsvp.note, "007");

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn delete_reservation_should_work() -> Result<(), Error> {
        let (rsvp, manager) = make_alice_reservation(&migrated_pool).await;

        manager.delete(rsvp.id).await?;

        let err = manager.get(rsvp.id).await.unwrap_err();

        // println!("The error: {:?}", err);

        assert_eq!(err, Error::NotFound);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn get_reservation_should_work() -> Result<(), Error> {
        let (rsvp, manager) = make_alice_reservation(&migrated_pool).await;

        let rsvp1 = manager.get(rsvp.id).await?;

        assert_eq!(rsvp, rsvp1);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn query_reservations_should_work() -> Result<(), Error> {
        let (rsvp, manager) = make_james_reservation(&migrated_pool).await;

        let query = ReservationQueryBuilder::default()
            .user_id("james id")
            .start("2022-10-25T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2022-12-31T00:00:00-0700".parse::<Timestamp>().unwrap())
            .status(ReservationStatus::Pending)
            .build()
            .unwrap();

        let rsvps = manager.query(query).await?;

        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvps[0], rsvp);

        let query = ReservationQueryBuilder::default()
            .user_id("james id")
            .start("2022-10-25T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2022-12-31T00:00:00-0700".parse::<Timestamp>().unwrap())
            .status(ReservationStatus::Confirmed)
            .build()
            .unwrap();

        let rsvps = manager.query(query.clone()).await?;

        assert_eq!(rsvps.len(), 0);

        let _rsvp = manager.change_status(rsvp.id).await?;
        let rsvps = manager.query(query).await?;

        assert_eq!(rsvps.len(), 1);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn filter_reservations_should_work() -> Result<(), Error> {
        let (_rsvp, manager) = make_james_reservation(&migrated_pool).await;

        let filter = ReservationFilterBuilder::default()
            .user_id("james id")
            .status(ReservationStatus::Pending)
            .build()
            .unwrap();

        let (pager, rsvps) = manager.filter(filter).await.unwrap();

        assert_eq!(pager.prev, -1);
        assert_eq!(pager.next, -1);
        assert_eq!(rsvps.len(), 1);

        Ok(())
    }

    async fn make_alice_reservation(pool: &PgPool) -> (abi::Reservation, ReservationManager) {
        make_reservation(
            pool,
            "alice id",
            "Ocean view room 518",
            "2022-12-25T15:00:00-0700",
            "2022-12-30T00:00:00-0700",
            "I'll arrive at 3pm, Please help to upgrade to execuitive room if possible.",
        )
        .await
    }

    async fn make_james_reservation(pool: &PgPool) -> (abi::Reservation, ReservationManager) {
        make_reservation(
            pool,
            "james id",
            "Ocean view room 5018",
            "2022-12-25T15:00:00-0700",
            "2022-12-30T00:00:00-0700",
            "I'll arrive at 3pm, Please help to upgrade to execuitive room if possible.",
        )
        .await
    }

    async fn make_reservation(
        pool: &PgPool,
        uid: &str,
        rid: &str,
        start: &str,
        end: &str,
        note: &str,
    ) -> (abi::Reservation, ReservationManager) {
        let manager = ReservationManager::new(pool.clone());

        let rsvp = abi::Reservation::new_pending(
            uid,
            rid,
            start.parse().unwrap(),
            end.parse().unwrap(),
            note,
        );

        (manager.reserve(rsvp).await.unwrap(), manager)
    }
}
