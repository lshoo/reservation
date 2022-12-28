use async_trait::async_trait;

use crate::{Error, ReservationId, ReservationManager, Rsvp};

use chrono::{DateTime, Utc};
use sqlx::{postgres::types::PgRange, types::Uuid, PgPool, Row};

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, Error> {
        rsvp.validate()?;

        let timespan: PgRange<DateTime<Utc>> = rsvp.get_timespan().into();

        let status = abi::ReservationStatus::from_i32(rsvp.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let id: Uuid = sqlx::query(
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

        rsvp.id = id.to_string();

        Ok(rsvp)
    }

    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        let id: Uuid = Uuid::parse_str(&id).map_err(|_| Error::InvalidReservationId(id.clone()))?;
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
        let id: Uuid = Uuid::parse_str(&id).map_err(|_| Error::InvalidReservationId(id.clone()))?;
        let rsvp: abi::Reservation =
            sqlx::query_as("UPDATE rsvp.reservations SET note = $1 where id = $2 RETURNING *")
                .bind(note)
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        Ok(rsvp)
    }

    async fn delete(&self, id: ReservationId) -> Result<(), Error> {
        let id: Uuid = Uuid::parse_str(&id).map_err(|_| Error::InvalidReservationId(id.clone()))?;
        sqlx::query("DELETE FROM rsvp.reservations WHERE id = $1 ")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        let id: Uuid = Uuid::parse_str(&id).map_err(|_| Error::InvalidReservationId(id.clone()))?;
        let rsvp: abi::Reservation =
            sqlx::query_as("SELECT * FROM rsvp.reservations WHERE id = $1 ")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        Ok(rsvp)
    }

    async fn query(&self, _query: abi::ReservationQuery) -> Result<Vec<abi::Reservation>, Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use abi::ReservationStatus;

    use super::*;

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() -> Result<(), Error> {
        let (rsvp, _) = make_james_reservation(&migrated_pool).await;
        assert!(!rsvp.id.is_empty());

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

        let rsvp = manager.change_status(rsvp.id.clone()).await?;

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

        manager.delete(rsvp.id.clone()).await?;

        let err = manager.get(rsvp.id).await.unwrap_err();

        // println!("The error: {:?}", err);

        assert_eq!(err, Error::NotFound);

        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn get_reservation_should_work() -> Result<(), Error> {
        let (rsvp, manager) = make_alice_reservation(&migrated_pool).await;

        let rsvp1 = manager.get(rsvp.id.clone()).await?;

        assert_eq!(rsvp, rsvp1);

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
