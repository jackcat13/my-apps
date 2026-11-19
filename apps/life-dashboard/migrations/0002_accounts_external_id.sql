-- Separates the connector's identity key for an account from its display label: `name` used
-- to double as both (the value a refresh matched on *and* the only string shown to the
-- user), so a future rename UI would have nowhere to put a label without breaking matching.

ALTER TABLE accounts ADD COLUMN external_id TEXT NOT NULL DEFAULT '';
UPDATE accounts SET external_id = name WHERE external_id = '';

-- Enforces the same uniqueness `accounts::upsert` already assumed at the application level.
CREATE UNIQUE INDEX accounts_connection_external_id ON accounts (connection_id, external_id);
