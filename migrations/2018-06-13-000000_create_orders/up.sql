CREATE SEQUENCE order_id_seq;
CREATE TABLE orders (
    id                          order_id PRIMARY KEY DEFAULT nextval('order_id_seq'),
    customer_id                 user_id NOT NULL,
    product                     product_id NOT NULL,
    price                       FLOAT NOT NULL,
    quantity                    quantity NOT NULL,
    subtotal                    FLOAT NOT NULL,
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
    creation_date               DATETIME NOT NULL,
    status                      VARCHAR,
    payment_status              BOOLEAN,
    delivery_company            VARCHAR NOT NULL,
    customer_comments           VARCHAR
);

CREATE UNIQUE INDEX order_id on orders (id);
CREATE INDEX order_user on orders (user_id);
