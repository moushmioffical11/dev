-- Add migration script here
CREATE TABLE user_data (
    user_id UUID PRIMARY KEY
        REFERENCES users(id)
        ON DELETE CASCADE,

    user_events JSONB NOT NULL DEFAULT '[]'::jsonb,

    event_results JSONB NOT NULL DEFAULT '[]'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT chk_user_events_array
        CHECK (jsonb_typeof(user_events) = 'array'),

    CONSTRAINT chk_event_results_array
        CHECK (jsonb_typeof(event_results) = 'array')
);