ALTER TABLE orders ADD COLUMN uuid uuid;

UPDATE orders SET uuid = uuid_generate_v4();

CREATE UNIQUE INDEX IF NOT EXISTS orders_orders_uuid_idx ON orders (uuid);

ALTER TABLE orders ALTER COLUMN uuid SET NOT NULL;
