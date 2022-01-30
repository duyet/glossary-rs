CREATE TABLE IF NOT EXISTS glossary_history (
  id         UUID PRIMARY KEY,
  term       VARCHAR(255) NOT NULL,
  definition TEXT NOT NULL,
  revision   INTEGER NOT NULL DEFAULT 0,
  who        VARCHAR(255),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  glossary_id   UUID                    NOT NULL REFERENCES glossary (id)
);
