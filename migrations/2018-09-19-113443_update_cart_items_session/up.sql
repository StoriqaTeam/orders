ALTER TABLE cart_items_session ADD COLUMN pre_order BOOLEAN NOT NULL DEFAULT 'f';
ALTER TABLE cart_items_session ADD COLUMN pre_order_days INTEGER NOT NULL DEFAULT 0;
