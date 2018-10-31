ALTER TABLE cart_items_session ADD COLUMN delivery_method_id JSONB;
ALTER TABLE cart_items_user ADD COLUMN delivery_method_id JSONB;
