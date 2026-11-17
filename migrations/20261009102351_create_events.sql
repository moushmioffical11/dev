-- Add migration script here
CREATE TABLE events (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),

    provider            text NOT NULL CHECK (provider IN ('ifinish', 'mysamay')),
    event_id            text NOT NULL,
    event_code          text,

    event_name          text NOT NULL,
    event_type          text,
    event_date          date,

    organizer           jsonb NOT NULL DEFAULT '{}',
    dates               jsonb NOT NULL DEFAULT '{}',
    location            jsonb NOT NULL DEFAULT '{}',
    links               jsonb NOT NULL DEFAULT '{}',
    images              jsonb NOT NULL DEFAULT '{}',
    details             jsonb NOT NULL DEFAULT '{}',
    prices              jsonb NOT NULL DEFAULT '[]',

    raw_data            jsonb,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    UNIQUE (provider, event_id)
);