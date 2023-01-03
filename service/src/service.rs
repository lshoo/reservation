use abi::{
    reservation_service_server::ReservationService, CancelRequest, CancelResponse, Config,
    ConfirmRequest, ConfirmResponse, Error, FilterResponse, GetRequest, GetResponse, ListenRequest,
    QueryRequest, ReservationFilter, ReserveRequest, ReserveResponse, UpdateRequest,
    UpdateResponse,
};

use reservation::{ReservationManager, Rsvp};
use tonic::{async_trait, Request, Response, Status};

use crate::{ReservationStream, RsvpService};

impl RsvpService {
    pub async fn from_config(config: &Config) -> Result<Self, Error> {
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

#[cfg(test)]
mod tests {

    use std::{ops::Deref, sync::Arc, thread};

    use abi::Reservation;
    use lazy_static::lazy_static;
    use sqlx::{types::Uuid, Connection, Executor, PgConnection};
    use tokio::runtime::Runtime;

    use super::*;

    struct TestConfig {
        config: Arc<Config>,
    }

    lazy_static! {
        /// This is an example for using doc comment attributes
        static ref RT: Runtime = Runtime::new().unwrap();
    }

    impl TestConfig {
        pub fn new() -> Self {
            let mut config = Config::load("../service/fixtures/config.yml").unwrap();
            // let old_url = config.db.url();
            let uuid = Uuid::new_v4();
            let dbname = format!("test_{}", uuid);
            // let url = config.db.url();

            config.db.dbname = dbname.clone();

            let server_url = config.db.server_url();
            let url = config.db.url();

            // create database dbname
            thread::spawn(move || {
                RT.block_on(async move {
                    // use server url to create database
                    let mut conn = PgConnection::connect(&server_url).await.unwrap();
                    conn.execute(format!(r#"CREATE DATABASE "{}""#, dbname).as_str())
                        .await
                        .expect("Error while create database");

                    let mut conn = PgConnection::connect(&url).await.unwrap();
                    sqlx::migrate!("../migrations")
                        .run(&mut conn)
                        .await
                        .unwrap();
                });
            })
            .join()
            .expect("failed to create database");

            Self {
                config: Arc::new(config),
            }
        }
    }

    impl Deref for TestConfig {
        type Target = Config;
        fn deref(&self) -> &Self::Target {
            &self.config
        }
    }

    impl Drop for TestConfig {
        fn drop(&mut self) {
            let server_url = self.config.db.server_url();

            let dbname = self.config.db.dbname.clone();

            thread::spawn(move || {
                RT.block_on(async move {
                    let mut conn = sqlx::PgConnection::connect(&server_url).await.unwrap();
                    // terminate existing connections
                    sqlx::query(&format!(r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = '{}'"#, dbname))
                        .execute(&mut conn)
                        .await
                        .expect("Terminate all other connections");

                    conn
                        .execute(format!(r#"DROP DATABASE "{}""#, dbname).as_str())
                        .await
                        .expect("Error while drop database");
                });
            })
            .join()
            .expect("failed to Drop database");
        }
    }

    #[tokio::test]
    async fn rpc_reserve_should_work() {
        // let config = Config::load("../service/fixtures/config.yml").unwrap();
        let config = TestConfig::new();
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
