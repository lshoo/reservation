mod service;

use std::pin::Pin;

use abi::Reservation;
use futures::Stream;
use reservation::ReservationManager;
use tonic::Status;

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;

pub struct RsvpService {
    manager: ReservationManager,
}
