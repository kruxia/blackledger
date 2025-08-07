-- Transaction table
CREATE SEQUENCE transaction_id_seq AS bigint;
CREATE TABLE transaction (
    id          bigint          PRIMARY KEY DEFAULT bigid('transaction_id_seq'),
    ledger_id   bigint          NOT NULL REFERENCES ledger(id),
    created     timestamptz(6)  NOT NULL DEFAULT now(),
    effective   timestamptz(6)  NOT NULL DEFAULT now(),
    memo        text,
    meta        jsonb
);

-- Prevent update or delete to an existing transaction record
CREATE FUNCTION transaction_no_update_delete() RETURNS trigger AS $$
    BEGIN
      RAISE EXCEPTION 'Transaction cannot be updated or deleted';
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER transaction_no_update_delete BEFORE UPDATE OR DELETE ON transaction
    FOR EACH ROW EXECUTE FUNCTION transaction_no_update_delete();