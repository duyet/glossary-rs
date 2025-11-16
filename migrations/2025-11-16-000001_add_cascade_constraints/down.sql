-- Revert ON DELETE CASCADE constraints back to default behavior

ALTER TABLE likes
DROP CONSTRAINT IF EXISTS likes_glossary_id_fkey,
ADD CONSTRAINT likes_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id);

ALTER TABLE glossary_history
DROP CONSTRAINT IF EXISTS glossary_history_glossary_id_fkey,
ADD CONSTRAINT glossary_history_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id);

ALTER TABLE glossary_tags
DROP CONSTRAINT IF EXISTS glossary_tags_glossary_id_fkey,
ADD CONSTRAINT glossary_tags_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id);

ALTER TABLE glossary_tags
DROP CONSTRAINT IF EXISTS glossary_tags_tag_id_fkey,
ADD CONSTRAINT glossary_tags_tag_id_fkey
    FOREIGN KEY (tag_id)
    REFERENCES tags(id);
