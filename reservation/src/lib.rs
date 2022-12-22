mod manager;

use abi::Error;
use async_trait::async_trait;
use sqlx::PgPool;

pub type ReservationId = String;
pub type UserId = String;
pub type ResourceId = String;

#[async_trait]
pub trait Rsvp {
    /// make a reservation
    async fn reserve(&self, rsvp: abi::Reservation) -> Result<abi::Reservation, Error>;
    /// change reservation status (if current status is pending, change it to confirmed)
    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, Error>;
    /// update note
    async fn update_note(&self, id: ReservationId, note: String)
        -> Result<abi::Reservation, Error>;
    /// delete reservation
    async fn delete_reservation(&self, id: ReservationId) -> Result<(), Error>;
    /// get reservation by id
    async fn get(&self, id: ReservationId) -> Result<Option<abi::Reservation>, Error>;
    // query reservations
    async fn query(&self, query: abi::ReservationQuery) -> Result<Vec<abi::Reservation>, Error>;
}

pub struct ReservationManager {
    pub pool: PgPool,
}
