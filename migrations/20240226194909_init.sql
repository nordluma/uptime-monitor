-- Add migration script here
CREATE TABLE IF NOT EXISTS websites (
    id SERIAL PRIMARY KEY,
    url VARCHAR NOT NULL,
    alias VARCHAR(75) NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS logs (
    id SERIAL PRIMARY KEY,
    website_id INT NOT NULL REFERENCES websites(id),
    status SMALLINT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT date_trunc('minute', CURRENT_TIMESTAMP),
    UNIQUE (website_id, created_at)
);
