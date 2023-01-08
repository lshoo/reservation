use abi::Config;
use sqlx_postgres_tester::TestPg;
use std::{ops::Deref, path::Path};

pub struct TestConfig {
    #[allow(dead_code)]
    tdb: TestPg,
    pub config: Config,
}

impl TestConfig {
    pub fn new(filename: impl AsRef<Path>) -> Self {
        let mut config = Config::load(filename).unwrap();
        let tdb = TestPg::new(config.db.server_url(), "../migrations");

        config.db.dbname = tdb.dbname.clone();

        Self { tdb, config }
    }

    pub fn with_server_port(port: u16) -> Self {
        let mut config = TestConfig::default();
        config.config.server.port = port;
        config
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new("fixtures/config.yml")
    }
}

impl Deref for TestConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}
