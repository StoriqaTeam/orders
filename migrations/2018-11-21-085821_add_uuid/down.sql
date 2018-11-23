DROP INDEX IF EXISTS orders_orders_uuid_idx;

ALTER TABLE orders DROP COLUMN uuid;
