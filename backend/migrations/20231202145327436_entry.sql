-- Entry table
CREATE TABLE entry (
    id          bigint    PRIMARY KEY DEFAULT bigid(),
    ledger_id   bigint    NOT NULL REFERENCES ledger(id),
    tx          bigint    NOT NULL REFERENCES transaction(id),
    acct        bigint    NOT NULL REFERENCES account(id),
    curr        varchar   NOT NULL REFERENCES currency(code),
    dr          decimal,
    cr          decimal,
    CHECK ((dr IS NOT NULL AND dr > 0 AND cr IS NULL)
        OR (cr IS NOT NULL AND cr > 0 AND dr IS NULL))
);

-- Add foreign key constraint from account.version to entry.id
ALTER TABLE account ADD CONSTRAINT account_version_fkey
    FOREIGN KEY (version) REFERENCES entry(id);

-- Prevent any update or delete to an existing entry record
CREATE FUNCTION entry_no_update_delete() RETURNS trigger AS $$
    BEGIN
      RAISE EXCEPTION 'Entry cannot be updated or deleted';
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER entry_no_update_delete BEFORE UPDATE OR DELETE ON entry
    FOR EACH ROW EXECUTE FUNCTION entry_no_update_delete();