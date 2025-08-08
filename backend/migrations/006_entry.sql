-- Entry table
CREATE SEQUENCE entry_id_seq AS bigint;
CREATE TABLE entry (
    id              bigint    PRIMARY KEY DEFAULT bigid('entry_id_seq'),
    ledger_id       bigint    NOT NULL REFERENCES ledger(id),
    transaction_id  bigint    NOT NULL REFERENCES transaction(id),
    account_id      bigint    NOT NULL REFERENCES account(id),
    currency            varchar   NOT NULL REFERENCES currency(code),
    debit           decimal,
    credit          decimal,
    CHECK ((debit IS NOT NULL AND debit > 0 AND credit IS NULL)
        OR (credit IS NOT NULL AND credit > 0 AND debit IS NULL))
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