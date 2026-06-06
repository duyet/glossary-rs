-- Add indexes for query performance optimization

-- Index on glossary.term for fast lookup and search
CREATE INDEX IF NOT EXISTS idx_glossary_term ON glossary(term);

-- Index on glossary.created_at for sorting and filtering by date
CREATE INDEX IF NOT EXISTS idx_glossary_created_at ON glossary(created_at DESC);

-- Index on glossary.updated_at for recently updated queries
CREATE INDEX IF NOT EXISTS idx_glossary_updated_at ON glossary(updated_at DESC);

-- Index on likes.glossary_id for fast like counting (already has FK, but explicit index helps)
CREATE INDEX IF NOT EXISTS idx_likes_glossary_id ON likes(glossary_id);

-- Index on likes.created_at for sorting likes by date
CREATE INDEX IF NOT EXISTS idx_likes_created_at ON likes(created_at DESC);

-- Index on glossary_history.glossary_id for fast history lookup
CREATE INDEX IF NOT EXISTS idx_glossary_history_glossary_id ON glossary_history(glossary_id);

-- Index on glossary_history.created_at for history sorting
CREATE INDEX IF NOT EXISTS idx_glossary_history_created_at ON glossary_history(created_at DESC);

-- Composite index for popular glossaries query (glossary_id + count)
-- This optimizes the list_popular_glossary query
CREATE INDEX IF NOT EXISTS idx_likes_glossary_id_created_at ON likes(glossary_id, created_at DESC);
