CREATE TABLE cart_items_session (
    id         UUID PRIMARY KEY,
    session_id UUID NOT NULL,
    product_id INTEGER NOT NULL,
    quantity   INTEGER NOT NULL,
    store_id   INTEGER NOT NULL,
    comment    VARCHAR NOT NULL,

    CONSTRAINT cart_items_session_constraint UNIQUE (session_id, product_id)
);
ALTER TABLE cart_items RENAME TO cart_items_user;

CREATE UNIQUE INDEX cart_items_session_idx on cart_items_session (session_id, product_id);
