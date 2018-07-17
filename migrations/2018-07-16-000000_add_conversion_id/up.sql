ALTER TABLE orders ADD COLUMN conversion_id UUID NOT NULL DEFAULT uuid_generate_v4();
