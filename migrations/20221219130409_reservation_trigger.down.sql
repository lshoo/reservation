-- Add down migration script here
DROP TRIGGER reservations_trigger on rsvp.reservations;
DROP FUNCTION reservations_trigger();
DROP TABLE rsvp.reservation_changes CASCADE;
