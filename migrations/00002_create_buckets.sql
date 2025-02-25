CREATE TABLE IF NOT EXISTS buckets (
                                       id UUID PRIMARY KEY,
                                       name VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name, user_id)
    );