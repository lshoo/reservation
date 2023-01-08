use std::{
    pin::Pin,
    task::{Context, Poll},
};

use abi::{
    reservation_service_server::ReservationService, CancelRequest, CancelResponse, Config,
    ConfirmRequest, ConfirmResponse, Error, FilterRequest, FilterResponse, GetRequest, GetResponse,
    ListenRequest, QueryRequest, ReserveRequest, ReserveResponse, UpdateRequest, UpdateResponse,
};

use futures::Stream;
use reservation::{ReservationManager, Rsvp};
use tokio::sync::mpsc;
use tonic::{async_trait, Request, Response, Status};

use crate::{ReservationStream, RsvpService, TonicReceiverStream};

impl RsvpService {
    pub async fn from_config(config: &Config) -> Result<Self, Error> {
        ReservationManager::from_config(&config.db)
            .await
            .map(|m| RsvpService { manager: m })
    }
}

impl<T> Stream for TonicReceiverStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_recv(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> TonicReceiverStream<T> {
    pub fn new(inner: mpsc::Receiver<Result<T, Error>>) -> Self {
        Self { inner }
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
        request: Request<ConfirmRequest>,
    ) -> Result<Response<ConfirmResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.change_status(request.id).await?;
        Ok(Response::new(ConfirmResponse {
            reservation: Some(reservation),
        }))
    }

    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.update_note(request.id, request.note).await?;
        Ok(Response::new(UpdateResponse {
            reservation: Some(reservation),
        }))
    }

    async fn cancel(
        &self,
        request: Request<CancelRequest>,
    ) -> Result<Response<CancelResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.delete(request.id).await?;
        Ok(Response::new(CancelResponse {
            reservation: Some(reservation),
        }))
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let request = request.into_inner();
        let reservation = self.manager.get(request.id).await?;
        Ok(Response::new(GetResponse {
            reservation: Some(reservation),
        }))
    }

    /// Server streaming response type for the query method.
    // type queryStream: futures_core::Stream<Item = Result<Reservation, Status>>
    //     + Send
    //     + 'static;
    type queryStream = ReservationStream;

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::queryStream>, Status> {
        let request = request.into_inner();

        if request.query.is_none() {
            return Err(Status::invalid_argument("missing filter params"));
        }

        let rx = self.manager.query(request.query.unwrap()).await;
        let stream = TonicReceiverStream::new(rx);

        Ok(Response::new(Box::pin(stream)))
    }

    async fn filter(
        &self,
        request: Request<FilterRequest>,
    ) -> Result<Response<FilterResponse>, Status> {
        let request = request.into_inner();

        if request.filter.is_none() {
            return Err(Status::invalid_argument("missing filter params"));
        }

        let (pager, rsvps) = self.manager.filter(request.filter.unwrap()).await?;
        Ok(Response::new(FilterResponse {
            pager: Some(pager),
            reservations: rsvps,
        }))
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utils::TestConfig;
    use abi::Reservation;

    #[tokio::test]
    async fn rpc_reserve_should_work() {
        let config = TestConfig::default();
        let service = RsvpService::from_config(&config).await.unwrap();
        let reservation = Reservation::new_pending(
            "james id",
            "Oceam view 5018",
            "2021-10-01T10:10:10-0700".parse().unwrap(),
            "2021-10-08T10:10:10-0700".parse().unwrap(),
            "test rpc reserve api",
        );
        let request = tonic::Request::new(ReserveRequest {
            reservation: Some(reservation.clone()),
        });
        let response = service.reserve(request).await.unwrap();
        let reservation1 = response.into_inner().reservation;

        assert!(reservation1.is_some());
        let reservation1 = reservation1.unwrap();
        assert_eq!(reservation.user_id, reservation1.user_id);
        assert_eq!(reservation.resource_id, reservation1.resource_id);
        assert_eq!(reservation.start, reservation1.start);
        assert_eq!(reservation.end, reservation1.end);
        assert_eq!(reservation.status, reservation1.status);
        assert_eq!(reservation.note, reservation1.note);

        // TestConfig dropped here
    }
}
