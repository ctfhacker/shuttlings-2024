CREATE TABLE IF NOT EXISTS pagination (
    id CHAR(16) PRIMARY KEY,
    page INT NOT NULL DEFAULT 1,
    prev_id CHAR(16)
);
