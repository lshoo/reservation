use std::path::Path;

use abi::{reservation_service_server::ReservationServiceServer, Config};
use anyhow::{Ok, Result};
use reservation_service::RsvpService;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let filename = std::env::var("RESERVATION_CONFIG").unwrap_or_else(|_| {
        let p1 = Path::new("./reservation.yml");
        let path = shellexpand::tilde("~/.config/reservation.yml");
        let p2 = Path::new(path.as_ref());
        let p3 = Path::new("/etc/reservation.yml");

        let p = match (p1.exists(), p2.exists(), p3.exists()) {
            (true, _, _) => p1,
            (_, true, _) => p2,
            (_, _, true) => p3,
            _ => panic!("config file not found"),
        };

        p.to_str().unwrap().to_string()
    });

    let config = Config::load(&filename)?;
    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    let svc = RsvpService::from_config(config).await?;
    let svc = ReservationServiceServer::new(svc);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
