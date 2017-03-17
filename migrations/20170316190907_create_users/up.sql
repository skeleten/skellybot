-- Your SQL goes here
CREATE TABLE users (
       id SERIAL PRIMARY KEY,
       discord_id BIGINT NOT NULL,
       last_seen timestamp DEFAULT NULL
)
