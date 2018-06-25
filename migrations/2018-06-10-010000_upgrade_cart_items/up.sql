ALTER TABLE cart_items
    ALTER COLUMN id TYPE cart_item_id,
    ALTER COLUMN user_id TYPE user_id,
    ALTER COLUMN product_id TYPE product_id,
    ALTER COLUMN store_id DROP DEFAULT,
    ALTER COLUMN store_id TYPE store_id,
    ALTER COLUMN quantity TYPE quantity;
