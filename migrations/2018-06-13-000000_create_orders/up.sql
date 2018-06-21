CREATE SEQUENCE order_id_seq;
CREATE TABLE orders (
    id                          order_id PRIMARY KEY,
    slug                        VARCHAR UNIQUE NOT NULL,
    customer                    user_id NOT NULL,
    product                     product_id NOT NULL,
    price                       NUMERIC NOT NULL,
    quantity                    quantity NOT NULL,
    subtotal                    NUMERIC NOT NULL,
    receiver_name               VARCHAR,
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
    creation_date               TIMESTAMP NOT NULL,
    state                       VARCHAR,
    state_data                  VARCHAR,
    payment_status              BOOLEAN,
    delivery_company            VARCHAR NOT NULL
);

CREATE INDEX order_user on orders (customer);
