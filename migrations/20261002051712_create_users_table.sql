-- Add migration script here
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    mobile_number VARCHAR(20) UNIQUE NOT NULL,
    address TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    age INT NOT NULL CHECK (age > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);