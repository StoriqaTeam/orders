ALTER TABLE orders ADD COLUMN company_package_id INTEGER;
ALTER TABLE orders ADD COLUMN delivery_price DOUBLE PRECISION NOT NULL DEFAULT 0;