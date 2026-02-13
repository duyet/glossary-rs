-- Remove performance indexes

DROP INDEX IF EXISTS idx_glossary_term;
DROP INDEX IF EXISTS idx_glossary_created_at;
DROP INDEX IF EXISTS idx_glossary_updated_at;
DROP INDEX IF EXISTS idx_likes_glossary_id;
DROP INDEX IF EXISTS idx_likes_created_at;
DROP INDEX IF EXISTS idx_glossary_history_glossary_id;
DROP INDEX IF EXISTS idx_glossary_history_created_at;
DROP INDEX IF EXISTS idx_likes_glossary_id_created_at;
