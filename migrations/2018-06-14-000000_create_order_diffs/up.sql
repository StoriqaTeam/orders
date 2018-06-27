CREATE TABLE order_diffs (
    id           UUID      PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent       UUID      NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    committer    INTEGER   NOT NULL,
    committed_at TIMESTAMP NOT NULL DEFAULT now()::timestamp,
    state        VARCHAR   NOT NULL,
    comment      VARCHAR
);
