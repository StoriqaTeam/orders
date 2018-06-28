DROP TABLE IF EXISTS orders;
CREATE SEQUENCE order_id_seq;
CREATE TABLE orders (
    id                          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug                        INTEGER UNIQUE NOT NULL DEFAULT nextval('order_id_seq'),
    store                       INTEGER NOT NULL,
    customer                    INTEGER NOT NULL,
    product                     INTEGER NOT NULL,
    price                       DOUBLE PRECISION NOT NULL,
    quantity                    INTEGER NOT NULL,
    receiver_name               VARCHAR NOT NULL,
    location                    POINT,
    administrative_area_level_1 VARCHAR,
    administrative_area_level_2 VARCHAR,
    country                     VARCHAR,
    locality                    VARCHAR,
    political                   VARCHAR,
    postal_code                 VARCHAR,
    route                       VARCHAR,
    street_number               VARCHAR,
    address                     VARCHAR,
    place_id                    VARCHAR,
    track_id                    VARCHAR,
    created_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    state                       VARCHAR NOT NULL,
    payment_status              BOOLEAN NOT NULL DEFAULT FALSE,
    delivery_company            VARCHAR
);

CREATE INDEX order_user on orders (customer);
