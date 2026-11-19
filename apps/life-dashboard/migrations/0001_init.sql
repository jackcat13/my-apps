-- Core data model: connections (institution credentials), accounts, positions,
-- transactions, snapshots, and a singleton preferences row.

CREATE TABLE connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    connector_type TEXT NOT NULL,
    -- Plaintext by design for v1: OS file permissions on the SQLite file are the
    -- security boundary, not an OS keyring (see apps/life-dashboard/AGENTS.md).
    credentials TEXT NOT NULL,
    last_sync TEXT
);

CREATE TABLE accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id INTEGER REFERENCES connections (id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    currency TEXT NOT NULL
);

CREATE TABLE positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    quantity REAL NOT NULL,
    cost_basis REAL NOT NULL,
    current_value REAL NOT NULL,
    native_currency TEXT NOT NULL
);

CREATE TABLE transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    occurred_at TEXT NOT NULL,
    transaction_type TEXT NOT NULL,
    amount REAL NOT NULL,
    asset TEXT
);

CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    taken_at TEXT NOT NULL,
    total_value REAL NOT NULL,
    currency TEXT NOT NULL
);

-- Singleton row (id is always 0): app-wide preferences.
CREATE TABLE preferences (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    display_currency TEXT NOT NULL DEFAULT 'EUR',
    theme TEXT NOT NULL DEFAULT 'system'
);

INSERT INTO preferences (id, display_currency, theme) VALUES (0, 'EUR', 'system');
