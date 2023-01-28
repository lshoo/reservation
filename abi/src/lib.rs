mod config;
mod error;
mod pb;
mod types;
mod utils;

pub use config::*;
pub use error::Error;
pub use pb::*;
pub use types::*;
pub use utils::*;

pub type ReservationId = i64;
pub type UserId = String;
pub type ResourceId = String;

pub trait Validator {
    fn validate(&self) -> Result<(), Error>;
}

pub trait Normalize: Validator {
    fn normalize(&mut self) -> Result<(), Error> {
        self.validate()?;
        self.do_normalize();
        Ok(())
    }

    fn do_normalize(&mut self);
}

pub trait ToSql {
    fn to_sql(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "reservation_status", rename_all = "lowercase")]
pub enum RsvpStatus {
    Unknown,
    Pending,
    Confirmed,
    Blocked,
}

impl Validator for ReservationId {
    fn validate(&self) -> Result<(), Error> {
        if self <= &0 {
            return Err(Error::InvalidReservationId(self.to_owned()));
        }

        Ok(())
    }
}
