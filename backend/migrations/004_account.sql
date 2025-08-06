-- Account table
CREATE SEQUENCE account_id_seq AS bigint;
CREATE TABLE account (
    id          bigint    PRIMARY KEY DEFAULT bigid('account_id_seq'),
    ledger_id   bigint    NOT NULL REFERENCES ledger(id),
    parent_id   bigint    REFERENCES account(id),
    name        varchar   NOT NULL,
    number      smallint,
    UNIQUE NULLS NOT DISTINCT (ledger_id, parent_id, name),
    UNIQUE (ledger_id, number),
    
    created     timestamptz(6)  NOT NULL DEFAULT now(),
    normal      varchar(2)      NOT NULL CHECK (normal in ('DR', 'CR')),
    version     bigint
);

-- Prevent the account "id" and "normal" columns being changed
CREATE FUNCTION account_no_update() RETURNS trigger AS $$
    BEGIN
      IF (OLD.id <> NEW.id) THEN
        RAISE EXCEPTION 'Account id cannot be updated';
      ELSIF (OLD.normal <> NEW.normal) THEN
        RAISE EXCEPTION 'Account normal type cannot be updated';
      END IF;
      RETURN NEW;
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER account_no_update BEFORE UPDATE ON account
    FOR EACH ROW EXECUTE FUNCTION account_no_update();