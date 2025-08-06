-- BigID function for generating unique IDs
CREATE SEQUENCE bigid_seq AS bigint;

CREATE OR REPLACE FUNCTION bigid() RETURNS bigint AS $$
  SELECT (
    (nextval('bigid_seq'))*1e3 + (random()*1e3)::bigint
  )::bigint;
$$ LANGUAGE SQL;