-- Ledger table
CREATE TABLE ledger (
    id        bigint          PRIMARY KEY DEFAULT bigid(),
    name      varchar         NOT NULL UNIQUE,
    created   timestamptz(6)  NOT NULL DEFAULT now()
);