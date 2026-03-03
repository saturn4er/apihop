-- Add extraction_rules column to saved_requests for request chaining
ALTER TABLE saved_requests ADD COLUMN extraction_rules TEXT;
