CREATE TYPE confidence_level AS ENUM ('VERIFIED', 'LIKELY', 'UNCONFIRMED');

ALTER TABLE links
    ADD COLUMN url_similarity     REAL,
    ADD COLUMN article_similarity REAL,
    ADD COLUMN confidence_score   REAL,
    ADD COLUMN confidence_level   confidence_level;
