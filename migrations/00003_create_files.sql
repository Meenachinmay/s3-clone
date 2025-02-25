CREATE TABLE IF NOT EXISTS files (
                                     id UUID PRIMARY KEY,
                                     filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100),
    size BIGINT NOT NULL,
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    storage_path VARCHAR(512) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(filename, bucket_id)
    );