CREATE TABLE exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_identifier TEXT NOT NULL,
    fetch_time INTEGER NOT NULL,
    snapshot_json TEXT NOT NULL,
    metadata_json TEXT,
    base_currency TEXT GENERATED ALWAYS AS (json_extract(snapshot_json, '$.base_currency')) VIRTUAL,
    rates_count INTEGER GENERATED ALWAYS AS (json_array_length(json_extract(snapshot_json, '$.rates'))) VIRTUAL,

    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

CREATE UNIQUE INDEX idx_source_fetch_time ON exchange_rates(source_identifier, fetch_time DESC);
CREATE INDEX idx_fetch_time ON exchange_rates(fetch_time);
CREATE INDEX idx_base_currency_time ON exchange_rates(base_currency, fetch_time DESC);
