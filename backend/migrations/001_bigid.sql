-- BigID function for generating unique monotonic non-guessable bigint IDs
CREATE OR REPLACE FUNCTION bigid(varchar) RETURNS bigint AS $$
  SELECT 
    (nextval($1) << 15) | floor(random() * 2^15)::bigint
  ;
$$ LANGUAGE SQL; 