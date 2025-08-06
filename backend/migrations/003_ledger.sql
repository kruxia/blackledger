-- Ledger table
CREATE SEQUENCE ledger_id_seq AS bigint;
CREATE TABLE ledger (
    id          bigint          PRIMARY KEY DEFAULT bigid('ledger_id_seq'),
    name        varchar         NOT NULL UNIQUE,
    created     timestamptz(6)  NOT NULL DEFAULT now()
);