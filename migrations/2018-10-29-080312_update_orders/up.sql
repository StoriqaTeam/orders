ALTER TABLE orders ADD COLUMN product_discount DOUBLE PRECISION;
ALTER TABLE orders ADD COLUMN coupon_percent INTEGER;
ALTER TABLE orders ADD COLUMN coupon_discount DOUBLE PRECISION;
ALTER TABLE orders ADD COLUMN total_amount DOUBLE PRECISION NOT NULL DEFAULT 0;
