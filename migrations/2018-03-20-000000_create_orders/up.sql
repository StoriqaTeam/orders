CREATE TABLE orders (
    id         SERIAL PRIMARY KEY,
    user_id    INTEGER NOT NULL,
    products   JSONB NOT NULL,
    state_id   VARCHAR NOT NULL,
    state_data JSONB NOT NULL
);

CREATE UNIQUE INDEX order_id on orders (id);
CREATE INDEX order_user on orders (user_id);
