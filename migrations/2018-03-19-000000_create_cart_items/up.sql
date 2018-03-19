CREATE TABLE cart_items (
    id         SERIAL PRIMARY KEY,
    user_id    INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity   INTEGER NOT NULL,

    CONSTRAINT item UNIQUE (user_id, product_id)
);

CREATE UNIQUE INDEX idx on cart_items (user_id, product_id)