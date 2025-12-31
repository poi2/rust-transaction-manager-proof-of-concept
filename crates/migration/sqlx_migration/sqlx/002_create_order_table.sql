-- Create order table in poc_for_sqlx schema
CREATE TABLE IF NOT EXISTS poc_for_sqlx.orders (
    id UUID PRIMARY KEY,
    item_id UUID NOT NULL,
    quantity INTEGER NOT NULL
);