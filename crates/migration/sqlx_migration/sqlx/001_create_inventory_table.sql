-- Create inventory table in poc_for_sqlx schema
CREATE SCHEMA IF NOT EXISTS poc_for_sqlx;

CREATE TABLE IF NOT EXISTS poc_for_sqlx.inventory (
    item_id UUID PRIMARY KEY,
    quantity INTEGER NOT NULL
);
