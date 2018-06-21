CREATE TABLE roles (
    id      role_id PRIMARY KEY,
    user_id user_id NOT NULL,
    name    VARCHAR NOT NULL,
    data    JSONB NOT NULL,

    CONSTRAINT role UNIQUE (user_id, name, data)
);
