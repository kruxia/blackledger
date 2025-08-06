-- Currency table
CREATE TABLE currency (
    code        varchar         PRIMARY KEY
                                CHECK (code ~ '^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$'),
    created     timestamptz(6)  NOT NULL DEFAULT now()
);