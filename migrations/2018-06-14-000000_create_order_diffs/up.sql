CREATE TABLE order_diffs (
    id        order_diff_id PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent    order_id      NOT NULL,
    committer user_id       NOT NULL,
    datetime  DATE          NOT NULL,
    status    VARCHAR       NOT NULL,
    comment   VARCHAR
);
