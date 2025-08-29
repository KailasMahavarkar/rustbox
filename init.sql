-- Database initialization script for Rustbox API

-- Create database if it doesn't exist
-- This is handled by the PostgreSQL container

-- Create extensions if needed
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- The tables will be created by SQLAlchemy models
-- This file is here for any additional database setup if needed