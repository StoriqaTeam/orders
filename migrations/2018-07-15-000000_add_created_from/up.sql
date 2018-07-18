ALTER TABLE orders ADD COLUMN created_from UUID NOT NULL DEFAULT uuid_generate_v4();
