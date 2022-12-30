-- Add up migration script here
-- if user_id is null, find all reservations within during for the resource
-- if resource_id is null, find all reservations within during for the user
-- if both are null, find all reservations within during
-- if both set, find all reservations within during for the resource and user
CREATE OR REPLACE FUNCTION rsvp.query(
    uid text,
    rid text,
    during TSTZRANGE,
    status rsvp.reservation_status,
    page integer DEFAULT 1,
    is_desc bool DEFAULT FALSE,
    page_size integer DEFAULT 10
) RETURNS TABLE (LIKE rsvp.reservations) AS $$
DECLARE
    _sql text;
BEGIN

    IF page_size < 10 OR page_size > 100 THEN
        page_size := 10;
    END IF;

    IF page < 1 THEN
        page := 1;
    END IF;

    -- format the query based on parameters
    _sql := format('SELECT * FROM rsvp.reservations WHERE %L @> timespan AND status = %L AND %s ORDER BY lower(timespan) %s LIMIT %L::integer OFFSET %L::integer',
        during,
        status,
        CASE
            WHEN uid IS NULL AND rid is NULL THEN 'TRUE'
            WHEN uid is NULL THEN 'resource_id = ' || quote_literal(rid)
            WHEN rid is NULL THEN 'user_id = ' || quote_literal(uid)
            ELSE 'user_id = ' || quote_literal(uid) || ' AND resource_id = ' || quote_literal(rid)
        END,
        CASE
            WHEN is_desc THEN 'DESC'
            ELSE 'ASC'
        END,
        page_size,
        (page - 1) * page_size
    );

    --log the sql
    RAISE NOTICE '%', _sql;

    -- execute the query
    RETURN QUERY EXECUTE _sql;
END;
$$ LANGUAGE plpgsql;

-- Should filter 2 more items, one for starting, on for ending.
-- If starting existing, then have previous page.
-- If ending existing, then have next page.
CREATE OR REPLACE FUNCTION rsvp.filter(
    uid text,
    rid text,
    status rsvp.reservation_status,
    cursor bigint DEFAULT 0,
    is_desc bool DEFAULT FALSE,
    page_size integer DEFAULT 10
) RETURNS TABLE (LIKE rsvp.reservations) AS $$
DECLARE
    _sql text;
    _offset bigint;
BEGIN
    IF page_size < 10 OR page_size > 100 THEN
        page_size := 10;
    END IF;

    -- if cursor is NULL or cursor less than 0, set it to 0 if is_desc is false, or to 2^63 -1 if is_desc is true.
    IF cursor IS NULL OR cursor < 0 THEN
        IF is_desc THEN
            cursor := 9223372036854775807;
        ELSE
            cursor := 0;
        END IF;
    END IF;

    -- format the query based on parameters
    _sql := format('SELECT * FROM rsvp.reservations WHERE %s AND status = %L AND %s ORDER BY id %s LIMIT %L::integer ',
        CASE
            WHEN is_desc THEN 'id <= ' || cursor
            ELSE 'id >= ' || cursor
        END,
        status,
        CASE
            WHEN uid IS NULL AND rid is NULL THEN 'TRUE'
            WHEN uid is NULL THEN 'resource_id = ' || quote_literal(rid)
            WHEN rid is NULL THEN 'user_id = ' || quote_literal(uid)
            ELSE 'user_id = ' || quote_literal(uid) || ' AND resource_id = ' || quote_literal(rid)
        END,
        CASE
            WHEN is_desc THEN 'DESC'
            ELSE 'ASC'
        END,
        page_size + 1
    );

    --log the sql
    RAISE NOTICE '%', _sql;

    -- execute the query
    RETURN QUERY EXECUTE _sql;
END;
$$ LANGUAGE plpgsql;
