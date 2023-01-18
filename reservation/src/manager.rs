use std::collections::VecDeque;

use abi::{convert_to_utc_time, DbConfig, Normalize, ToSql, Validator};
use async_trait::async_trait;
use tracing::{info, warn};

use crate::{Error, ReservationId, ReservationManager, Rsvp};
use futures::StreamExt;
use sqlx::{postgres::PgPoolOptions, Either, PgPool, Row};
use tokio::sync::mpsc::{self};

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn from_config(config: &DbConfig) -> Result<Self, abi::Error> {
        let url = config.url();
        let pool = PgPoolOptions::default()
            .max_connections(config.max_connections)
            .connect(&url)
            .await?;

        Ok(Self::new(pool))
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

    async fn delete(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        id.validate()?;
        let rsvp = sqlx::query_as("DELETE FROM rsvp.reservations WHERE id = $1 RETURNING *")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        Ok(rsvp)
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

    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> mpsc::Receiver<Result<abi::Reservation, Error>> {
        let user_id = string_to_opt(&query.user_id);
        let resource_id = string_to_opt(&query.resource_id);
        let start = query.start.map(convert_to_utc_time);
        let end = query.end.map(convert_to_utc_time);
        let status = abi::ReservationStatus::from_i32(query.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let pool = self.pool.clone();

        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            let mut rsvps = sqlx::query_as(
                "SELECT * FROM rsvp.query($1, $2, $3, $4, $5::rsvp.reservation_status, $6)",
            )
            .bind(user_id)
            .bind(resource_id)
            .bind(start)
            .bind(end)
            .bind(status.to_string())
            .bind(query.desc)
            .fetch_many(&pool);

            while let Some(ret) = rsvps.next().await {
                match ret {
                    Ok(Either::Left(r)) => {
                        info!("Query result: {:?}", r);
                    }
                    Ok(Either::Right(r)) => {
                        if tx.send(Ok(r)).await.is_err() {
                            // rx is dropped, so client disconnected
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Query error: {:?}", e);
                        if tx.send(Err(e.into())).await.is_err() {
                            // rx is dropped.
                            break;
                        }
                    }
                }
            }
        });

        rx
    }

    async fn filter(
        &self,
        mut filter: abi::ReservationFilter,
    ) -> Result<(abi::FilterPager, Vec<abi::Reservation>), Error> {
        filter.normalize()?;

        let sql = filter.to_sql()?;

        let rsvps: Vec<abi::Reservation> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;

        let mut rsvps: VecDeque<abi::Reservation> = rsvps.into_iter().collect();

        let pager = filter.get_pager(&mut rsvps);

        Ok((pager, rsvps.into_iter().collect()))
    }
}

fn string_to_opt(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.into())
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

        let mut rx = manager.query(query).await;

        assert_eq!(rx.recv().await, Some(Ok(rsvp.clone())));
        assert_eq!(rx.recv().await, None);

        let query = ReservationQueryBuilder::default()
            .user_id("james id")
            .start("2022-10-25T15:00:00-0700".parse::<Timestamp>().unwrap())
            .end("2022-12-31T00:00:00-0700".parse::<Timestamp>().unwrap())
            .status(ReservationStatus::Confirmed)
            .build()
            .unwrap();

        let mut rx = manager.query(query.clone()).await;

        assert_eq!(rx.recv().await, None);

        let rsvp = manager.change_status(rsvp.id).await?;
        let mut rx = manager.query(query).await;

        assert_eq!(rx.recv().await, Some(Ok(rsvp)));

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

        assert_eq!(pager.prev, None);
        assert_eq!(pager.next, None);
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
