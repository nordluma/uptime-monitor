-- Add migration script here
ALTER TABLE logs
ADD website_alias VARCHAR(75) NOT NULL REFERENCES websites(alias);
