ALTER TABLE orders DROP COLUMN currency_id;
ALTER TABLE orders ADD COLUMN currency VARCHAR NOT NULL DEFAULT 'STQ';