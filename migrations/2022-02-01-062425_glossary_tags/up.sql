CREATE TABLE IF NOT EXISTS tags (
  id UUID PRIMARY KEY,
  tag_name VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS glossary_tags (
  glossary_id UUID NOT NULL REFERENCES glossary(id),
  tag_id UUID NOT NULL REFERENCES tags(id),
  PRIMARY KEY (glossary_id, tag_id)
);
