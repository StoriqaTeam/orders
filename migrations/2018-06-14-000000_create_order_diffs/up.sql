CREATE TABLE order_diffs (
    id        UUID    PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent    UUID    NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    committer INTEGER NOT NULL,
    datetime  DATE    NOT NULL,
    status    VARCHAR NOT NULL,
    comment   VARCHAR
);
