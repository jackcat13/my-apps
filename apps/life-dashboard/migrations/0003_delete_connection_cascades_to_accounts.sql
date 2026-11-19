-- Deleting a connection used to orphan its accounts (connection_id set to NULL), keeping
-- their positions/snapshots/transactions around forever with no way to remove them from the
-- UI. Switch to cascading the delete instead: removing a connection now removes its accounts
-- too, which already cascades further to their positions/snapshots/transactions. SQLite can't
-- alter a foreign key's ON DELETE action in place, so the table is rebuilt.

CREATE TABLE accounts_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id INTEGER REFERENCES connections (id) ON DELETE CASCADE,
    external_id TEXT NOT NULL,
    name TEXT NOT NULL,
    currency TEXT NOT NULL
);

INSERT INTO accounts_new (id, connection_id, external_id, name, currency)
SELECT id, connection_id, external_id, name, currency FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;

CREATE UNIQUE INDEX accounts_connection_external_id ON accounts (connection_id, external_id);
