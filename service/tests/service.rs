#[path = "../src/test_utils.rs"]
mod test_utils;
use std::time::Duration;

use abi::{
    reservation_service_client::ReservationServiceClient, ConfirmRequest, FilterRequest,
    FilterResponse, Reservation, ReservationFilterBuilder, ReservationStatus, ReserveRequest,
};
use reservation_service::start_server;
use test_utils::TestConfig;

#[tokio::test]
async fn grpc_server_should_work() {
    let tconfig = TestConfig::default();
    let config = tconfig.config.clone();
    let config_cloned = config.clone();
    tokio::spawn(async move {
        start_server(&config_cloned).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("the server url: {:?}", config.server.url(false));
    let mut client = ReservationServiceClient::connect(config.server.url(false))
        .await
        .unwrap();

    // first reservte a reservation
    let mut rsvp = Reservation::new_pending(
        "james id",
        "Ocean view room 5018",
        "2022-12-25T15:00:00-0700".parse().unwrap(),
        "2022-12-30T00:00:00-0700".parse().unwrap(),
        "test service in grpc",
    );

    let ret = client
        .reserve(ReserveRequest::new(rsvp.clone()))
        .await
        .unwrap()
        .into_inner()
        .reservation
        .unwrap();
    rsvp.id = ret.id;
    assert_eq!(ret, rsvp);

    // then try to reserve a conflicating reservation
    let rsvp2 = Reservation::new_pending(
        "james id",
        "Ocean view room 5018",
        "2022-12-25T15:00:00-0700".parse().unwrap(),
        "2022-12-30T00:00:00-0700".parse().unwrap(),
        "test service in grpc2",
    );
    let ret2 = client.reserve(ReserveRequest::new(rsvp2)).await;
    assert!(ret2.is_err());
    // assert_eq!(
    //     ret2.unwrap_err().to_string(),
    //     "rpc error: code = InvalidArgument desc ",
    // );

    // then confirm the first reservation
    let ret3 = client
        .confirm(ConfirmRequest::new(rsvp.id))
        .await
        .unwrap()
        .into_inner()
        .reservation
        .unwrap();
    assert_eq!(ret3.status, ReservationStatus::Confirmed as i32);

    // then make 100 reservations without confliction
    for i in 0..100 {
        let mut rsvp = Reservation::new_pending(
            "james id",
            format!("Ocean view room {}", i),
            "2022-12-25T15:00:00-0700".parse().unwrap(),
            "2022-12-30T00:00:00-0700".parse().unwrap(),
            format!("test service in grpc with id {}", i),
        );

        let ret = client
            .reserve(ReserveRequest::new(rsvp.clone()))
            .await
            .unwrap()
            .into_inner()
            .reservation
            .unwrap();
        rsvp.id = ret.id;
        assert_eq!(ret, rsvp);
    }

    // then filter by user
    let filter = ReservationFilterBuilder::default()
        .user_id("james id")
        .status(abi::ReservationStatus::Pending as i32)
        .build()
        .unwrap();
    let FilterResponse {
        pager,
        reservations,
    } = client
        .filter(FilterRequest::new(filter.clone()))
        .await
        .unwrap()
        .into_inner();

    let pager = pager.unwrap();

    // assert_eq!(pager.next, filter.page_size + 1 );
    assert_eq!(pager.prev, -1);

    println!("rsvps lens: {}", reservations.len());
    // assert_eq!(reservations.len(), filter.page_size as usize);

    // // then get next page
    // let mut next_filter = filter.clone();
    // next_filter.cursor = pager.next;

    // let FilterResponse {
    //     pager,
    //     reservations,
    // } = client
    //     .filter(FilterRequest::new(next_filter.clone()))
    //     .await
    //     .unwrap()
    //     .into_inner();

    // let pager = pager.unwrap();

    // assert_eq!(pager.next, next_filter.cursor + filter.page_size);
    // assert_eq!(pager.prev, next_filter.cursor - 1);

    // assert_eq!(reservations.len(), filter.page_size as usize);
}
