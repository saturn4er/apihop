-- Add GraphQL-specific fields to saved_requests
ALTER TABLE saved_requests ADD COLUMN graphql_query TEXT;
ALTER TABLE saved_requests ADD COLUMN graphql_variables TEXT;
ALTER TABLE saved_requests ADD COLUMN graphql_operation_name TEXT;
