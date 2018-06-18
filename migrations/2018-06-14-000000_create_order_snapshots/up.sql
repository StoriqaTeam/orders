CREATE ENUM change (
    address,
    status,
);
CREATE TABLE order_snapshots (
    id        UUID     NOT NULL,
    parent    order_id NOT NULL,
    committer user_id  NOT NULL,
    datetime  DATE     NOT NULL,
    change    VARCHAR  NOT NULL,
    diff      JSONB    NOT NULL
);
