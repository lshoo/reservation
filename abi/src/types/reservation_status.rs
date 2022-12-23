use crate::{ReservationStatus, RsvpStatus};
use std::fmt;

impl fmt::Display for ReservationStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReservationStatus::Pending => write!(f, "pending"),
            ReservationStatus::Blocked => write!(f, "blocked"),
            ReservationStatus::Confirmed => write!(f, "confirmed"),
            ReservationStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<RsvpStatus> for ReservationStatus {
    fn from(rsvp: RsvpStatus) -> Self {
        match rsvp {
            RsvpStatus::Unknown => Self::Unknown,
            RsvpStatus::Pending => Self::Pending,
            RsvpStatus::Confirmed => Self::Confirmed,
            RsvpStatus::Blocked => Self::Blocked,
        }
    }
}
