mod conflict;

use sqlx::postgres::PgDatabaseError;

pub use conflict::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Database error {0}")]
    DbError(sqlx::Error),

    #[error("reservation not found")]
    NotFound,

    #[error("config file not found")]
    ConfigReadError,

    #[error("config parse error")]
    ConfigParseError,

    #[error("Conflict reservation: {0}")]
    ConflictReservation(String),

    #[error("invalid start or end time for the reservation")]
    InvalidTime,

    #[error("invalid reservation id {0}")]
    InvalidReservationId(i64),

    #[error("invalid user id {0}")]
    InvalidUserId(String),

    #[error("invalid resource id {0}")]
    InvalidResourceId(String),

    #[error("invalid page size {0}")]
    InvalidPageSize(i64),

    #[error("invalid cursor {0}")]
    InvalidCursor(i64),

    #[error("invalid reservation status {0}")]
    InvalidStatus(i32),

    #[error("unknown error")]
    Unknown,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::DbError(_), Self::DbError(_)) => true,
            (Self::NotFound, Self::NotFound) => true,
            (Self::ConflictReservation(v1), Self::ConflictReservation(v2)) => v1 == v2,
            (Self::InvalidTime, Self::InvalidTime) => true,
            (Self::InvalidReservationId(v1), Self::InvalidReservationId(v2)) => v1 == v2,
            (Self::InvalidUserId(v1), Self::InvalidUserId(v2)) => v1 == v2,
            (Self::InvalidResourceId(v1), Self::InvalidResourceId(v2)) => v1 == v2,
            (Self::Unknown, Self::Unknown) => true,
            _ => false,
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::Database(e) => {
                let err: &PgDatabaseError = e.downcast_ref();
                match (err.code(), err.schema(), err.table()) {
                    ("23P01", Some("rsvp"), Some("reservations")) => {
                        Error::ConflictReservation(err.detail().unwrap().to_string())
                    }
                    _ => Error::DbError(sqlx::Error::Database(e)),
                }
            }
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::DbError(e),
        }
    }
}

impl From<Error> for tonic::Status {
    fn from(e: Error) -> Self {
        match e {
            Error::DbError(_) | Error::ConfigReadError | Error::ConfigParseError => {
                tonic::Status::internal(e.to_string())
            }

            Error::ConflictReservation(info) => {
                tonic::Status::failed_precondition(format!("Conflict Reservation: {info:?}"))
            }

            Error::InvalidTime
            | Error::InvalidResourceId(_)
            | Error::InvalidUserId(_)
            | Error::InvalidReservationId(_)
            | Error::InvalidPageSize(_)
            | Error::InvalidCursor(_)
            | Error::InvalidStatus(_) => tonic::Status::invalid_argument(e.to_string()),

            Error::NotFound => {
                tonic::Status::not_found("No reservatoin found by the given condition")
            }

            Error::Unknown => tonic::Status::unknown("Unknown error"),
        }
    }
}
