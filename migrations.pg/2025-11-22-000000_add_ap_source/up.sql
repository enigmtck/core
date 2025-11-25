-- Add ap_source column to store original content format (e.g., Markdown)
-- This allows editing content in its original format rather than reverse-engineering from HTML
ALTER TABLE objects ADD COLUMN ap_source JSONB;
