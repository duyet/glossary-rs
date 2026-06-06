-- Add ON DELETE CASCADE to foreign key constraints for data integrity
-- This ensures referential integrity is maintained at the database level

-- Drop existing foreign key constraints and re-add with CASCADE
ALTER TABLE likes
DROP CONSTRAINT IF EXISTS likes_glossary_id_fkey,
ADD CONSTRAINT likes_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id)
    ON DELETE CASCADE;

ALTER TABLE glossary_history
DROP CONSTRAINT IF EXISTS glossary_history_glossary_id_fkey,
ADD CONSTRAINT glossary_history_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id)
    ON DELETE CASCADE;

-- Also fix the tags tables (even though unused, maintain consistency)
ALTER TABLE glossary_tags
DROP CONSTRAINT IF EXISTS glossary_tags_glossary_id_fkey,
ADD CONSTRAINT glossary_tags_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id)
    ON DELETE CASCADE;

ALTER TABLE glossary_tags
DROP CONSTRAINT IF EXISTS glossary_tags_tag_id_fkey,
ADD CONSTRAINT glossary_tags_tag_id_fkey
    FOREIGN KEY (tag_id)
    REFERENCES tags(id)
    ON DELETE CASCADE;
