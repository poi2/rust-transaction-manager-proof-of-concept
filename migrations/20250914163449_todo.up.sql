-- Add up migration script here
CREATE TABLE IF NOT EXISTS todo
(
    id          uuid    PRIMARY KEY,
    description TEXT    NOT NULL,
    done        BOOLEAN NOT NULL DEFAULT FALSE
);
