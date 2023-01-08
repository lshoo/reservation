mod service;

use std::pin::Pin;

use abi::{reservation_service_server::ReservationServiceServer, Config, Reservation};
use futures::Stream;
use reservation::ReservationManager;
use tokio::sync::mpsc;
use tonic::{transport::Server, Status};

#[cfg(test)]
pub mod test_utils;

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;

pub struct RsvpService {
    manager: ReservationManager,
}

pub struct TonicReceiverStream<T> {
    pub inner: mpsc::Receiver<Result<T, abi::Error>>,
}

pub async fn start_server(config: &Config) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    let svc = RsvpService::from_config(config).await?;
    let svc = ReservationServiceServer::new(svc);

    println!("Listening on {:?}", addr);
    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
