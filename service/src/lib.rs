use std::pin::Pin;

use abi::{
    reservation_service_server::ReservationService, CancelRequest, CancelResponse, Config,
    ConfirmRequest, ConfirmResponse, Error, FilterResponse, GetRequest, GetResponse, ListenRequest,
    QueryRequest, Reservation, ReservationFilter, ReserveRequest, ReserveResponse, UpdateRequest,
    UpdateResponse,
};
use futures::Stream;
use reservation::{ReservationManager, Rsvp};
use tonic::{async_trait, Request, Response, Status};

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;

pub struct RsvpService {
    manager: ReservationManager,
}

impl RsvpService {
    pub async fn from_config(config: Config) -> Result<Self, Error> {
        ReservationManager::from_config(&config.db)
            .await
            .map(|m| RsvpService { manager: m })
    }
}

#[async_trait]
impl ReservationService for RsvpService {
    async fn reserve(
        &self,
        request: Request<ReserveRequest>,
    ) -> Result<Response<ReserveResponse>, Status> {
        let request = request.into_inner();
        if request.reservation.is_none() {
            return Err(Status::invalid_argument("missing reservation"));
        }

        let reservation = self.manager.reserve(request.reservation.unwrap()).await?;

        Ok(Response::new(ReserveResponse {
            reservation: Some(reservation),
        }))
    }

    async fn confirm(
        &self,
        _request: Request<ConfirmRequest>,
    ) -> Result<Response<ConfirmResponse>, Status> {
        todo!()
    }

    async fn update(
        &self,
        _request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        todo!()
    }

    async fn cancel(
        &self,
        _request: Request<CancelRequest>,
    ) -> Result<Response<CancelResponse>, Status> {
        todo!()
    }

    async fn get(&self, _request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        todo!()
    }

    /// Server streaming response type for the query method.
    // type queryStream: futures_core::Stream<Item = Result<Reservation, Status>>
    //     + Send
    //     + 'static;
    type queryStream = ReservationStream;

    async fn query(
        &self,
        _request: Request<QueryRequest>,
    ) -> Result<Response<Self::queryStream>, Status> {
        todo!()
    }

    async fn filter(
        &self,
        _request: Request<ReservationFilter>,
    ) -> Result<Response<FilterResponse>, Status> {
        todo!()
    }

    /// Server streaming response type for the listen method.
    // type listenStream: futures_core::Stream<Item = Result<Reservation, Status>>
    //     + Send
    //     + 'static;
    type listenStream = ReservationStream;

    /// another system could monitor newly added/confirmed/canceled reservation
    async fn listen(
        &self,
        _request: Request<ListenRequest>,
    ) -> Result<Response<Self::listenStream>, Status> {
        todo!()
    }
}
