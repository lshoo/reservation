use async_trait::async_trait;

use crate::{ReservationError, ReservationId, ReservationManager, Rsvp};

use chrono::{DateTime, Utc};
use sqlx::{postgres::types::PgRange, types::Uuid, PgPool, Row};

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(
        &self,
        mut rsvp: abi::Reservation,
    ) -> Result<abi::Reservation, ReservationError> {
        if rsvp.start.is_none() || rsvp.end.is_none() {
            return Err(ReservationError::InvalidTime);
        }

        let start = abi::convert_to_utc_time(rsvp.start.clone().unwrap());
        let end = abi::convert_to_utc_time(rsvp.end.clone().unwrap());

        let timespan: PgRange<DateTime<Utc>> = (start..end).into();

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

    async fn change_status(
        &self,
        _id: ReservationId,
    ) -> Result<abi::Reservation, ReservationError> {
        todo!()
    }

    async fn update_note(
        &self,
        _id: ReservationId,
        _note: String,
    ) -> Result<abi::Reservation, ReservationError> {
        todo!()
    }

    async fn delete_reservation(&self, _id: ReservationId) -> Result<(), ReservationError> {
        todo!()
    }

    async fn get(&self, _id: ReservationId) -> Result<Option<abi::Reservation>, ReservationError> {
        todo!()
    }

    async fn query(
        &self,
        _query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, ReservationError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() -> Result<(), ReservationError> {
        let manager = ReservationManager::new(migrated_pool.clone());

        let rsvp = abi::Reservation::new_pending(
            "james id",
            "Ocean view room 518",
            "2022-12-25T15:00:00-0700".parse().unwrap(),
            "2022-12-30T00:00:00-0700".parse().unwrap(),
            "I'll arrive at 3pm, Please help to upgrade to execuitive room if possible.",
        );

        let rsvp = manager.reserve(rsvp).await?;
        assert!(!rsvp.id.is_empty());

        Ok(())
    }
}
