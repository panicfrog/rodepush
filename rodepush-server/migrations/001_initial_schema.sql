-- Initial database schema for RodePush
-- This migration creates the basic tables for applications, bundles, deployments, and diff packages

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Applications table
CREATE TABLE applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_key VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    description TEXT,
    owner VARCHAR(255),
    settings JSONB DEFAULT '{}'::jsonb
);

-- Create index on api_key for fast lookups
CREATE INDEX idx_applications_api_key ON applications(api_key);

-- Bundles table
CREATE TABLE bundles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID REFERENCES applications(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    platform VARCHAR(20) NOT NULL,
    metadata JSONB NOT NULL,
    storage_key VARCHAR(500) NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum VARCHAR(128) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for bundle lookups
CREATE INDEX idx_bundles_application_id ON bundles(application_id);
CREATE INDEX idx_bundles_version ON bundles(version);
CREATE INDEX idx_bundles_platform ON bundles(platform);
CREATE INDEX idx_bundles_created_at ON bundles(created_at);

-- Deployments table
CREATE TABLE deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID REFERENCES applications(id) ON DELETE CASCADE,
    bundle_id UUID REFERENCES bundles(id) ON DELETE CASCADE,
    environment VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    rollout_percentage INTEGER DEFAULT 100,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deployed_at TIMESTAMP WITH TIME ZONE,
    rolled_back_at TIMESTAMP WITH TIME ZONE,
    description TEXT,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Create indexes for deployment lookups
CREATE INDEX idx_deployments_application_id ON deployments(application_id);
CREATE INDEX idx_deployments_bundle_id ON deployments(bundle_id);
CREATE INDEX idx_deployments_environment ON deployments(environment);
CREATE INDEX idx_deployments_status ON deployments(status);
CREATE INDEX idx_deployments_created_at ON deployments(created_at);

-- Differential packages cache table
CREATE TABLE diff_packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_bundle_id UUID REFERENCES bundles(id) ON DELETE CASCADE,
    target_bundle_id UUID REFERENCES bundles(id) ON DELETE CASCADE,
    storage_key VARCHAR(500) NOT NULL,
    size_bytes BIGINT NOT NULL,
    compression_ratio FLOAT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    checksum VARCHAR(128) NOT NULL,
    platform VARCHAR(20) NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Create indexes for diff package lookups
CREATE INDEX idx_diff_packages_source_bundle_id ON diff_packages(source_bundle_id);
CREATE INDEX idx_diff_packages_target_bundle_id ON diff_packages(target_bundle_id);
CREATE INDEX idx_diff_packages_created_at ON diff_packages(created_at);
CREATE UNIQUE INDEX idx_diff_packages_source_target ON diff_packages(source_bundle_id, target_bundle_id);

-- Add constraints
ALTER TABLE deployments ADD CONSTRAINT chk_rollout_percentage CHECK (rollout_percentage >= 0 AND rollout_percentage <= 100);
ALTER TABLE deployments ADD CONSTRAINT chk_status CHECK (status IN ('pending', 'active', 'paused', 'rolled_back', 'failed'));
ALTER TABLE bundles ADD CONSTRAINT chk_platform CHECK (platform IN ('ios', 'android', 'both'));
ALTER TABLE diff_packages ADD CONSTRAINT chk_compression_ratio CHECK (compression_ratio >= 0.0 AND compression_ratio <= 1.0);
ALTER TABLE diff_packages ADD CONSTRAINT chk_platform CHECK (platform IN ('ios', 'android', 'both'));

-- Add triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_applications_updated_at BEFORE UPDATE ON applications
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column(); 