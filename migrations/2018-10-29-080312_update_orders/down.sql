ALTER TABLE orders DROP COLUMN IF EXISTS product_discount;
ALTER TABLE orders DROP COLUMN IF EXISTS coupon_percent;
ALTER TABLE orders DROP COLUMN IF EXISTS coupon_discount;
ALTER TABLE orders DROP COLUMN IF EXISTS total_amount;
