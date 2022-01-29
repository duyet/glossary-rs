CREATE TABLE IF NOT EXISTS glossary (
  id         UUID PRIMARY KEY,
  term       VARCHAR(255) NOT NULL,
  definition TEXT NOT NULL,
  revision   INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
  UNIQUE     (term)
);

CREATE TABLE IF NOT EXISTS likes
(
  id            UUID PRIMARY KEY        NOT NULL,
  created_at    TIMESTAMP DEFAULT now() NOT NULL,
  glossary_id   UUID                    NOT NULL REFERENCES glossary (id)
);

