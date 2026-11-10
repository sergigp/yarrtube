-- Every preferences save before Auto categorization context length stored
-- the old default, 4096, whether the user chose it or not: drop it so those
-- preferences read as Auto. A length saved from now on is kept as saved.
UPDATE preferences
SET data = json_remove(data, '$.categorization_context_length')
WHERE json_extract(data, '$.categorization_context_length') = 4096;
