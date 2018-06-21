CREATE TABLE order_diffs (
    id        order_diff_id NOT NULL,
    parent    order_id      NOT NULL,
    committer user_id       NOT NULL,
    datetime  DATE          NOT NULL,
    change    VARCHAR       NOT NULL,
    diff      JSONB         NOT NULL
);
