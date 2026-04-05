-- 1. EXTENSIONS
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "ltree";

-- 2. AUTHENTICATION TABLES (Users & Sessions)
CREATE TABLE users (
    username TEXT PRIMARY KEY, 
    name TEXT NOT NULL, 
    password TEXT NOT NULL
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(), 
    username TEXT NOT NULL REFERENCES users(username) ON DELETE CASCADE, 
    token TEXT NOT NULL, 
    created_at TIMESTAMPTZ DEFAULT NOW(), 
    expires_at TIMESTAMPTZ NOT NULL
);


-- 3. FILESYSTEM TYPES
-- 'folder' represents a directory.
-- 'file_link' represents a hardlink pointing to the actual image data.
CREATE TYPE entry_type AS ENUM ('folder', 'file_link');

-- 4. THE DATA TABLE (Inodes / SeaweedFS Pointers)
-- This table holds the actual physical file data. No names, no hierarchy here.
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_username TEXT NOT NULL REFERENCES users(username) ON UPDATE CASCADE,
    
    s3_fileid TEXT NOT NULL, 
    size_bytes BIGINT DEFAULT 0 CHECK (size_bytes <= 100000000), -- e.g., 100MB limit
    mime_type TEXT NOT NULL,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 5. THE HIERARCHY TABLE (Dentries / Folders & Shortcuts)
-- This table creates the folder structure and links names to the file data.
CREATE TABLE filesystem (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent_id UUID REFERENCES filesystem(id) ON DELETE CASCADE,
    
    name TEXT NOT NULL, -- User-facing name (spaces, emojis, etc. are safe here)
    type entry_type NOT NULL,
    
    -- Points to the actual file. MUST be NULL if this row is a folder.
    file_id UUID REFERENCES files(id) ON DELETE RESTRICT, 
    owner_username TEXT NOT NULL REFERENCES users(username) ON UPDATE CASCADE,
    
    path LTREE, -- The UUID-based fast query path for Postgres
    
    sort_order INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,

    -- Ensure folders don't point to files, and file_links ALWAYS point to files
    CONSTRAINT check_hardlink_logic CHECK (
        (type = 'folder' AND file_id IS NULL) OR 
        (type = 'file_link' AND file_id IS NOT NULL)
    )
);


-- 6. LTREE PATH GENERATION LOGIC (UUID Based)
CREATE OR REPLACE FUNCTION update_filesystem_path() RETURNS TRIGGER AS $$
DECLARE
    parent_path LTREE;
    formatted_id TEXT;
BEGIN
    -- Format UUID for ltree (ltree doesn't allow hyphens, so we swap them for underscores)
    formatted_id := replace(NEW.id::text, '-', '_');

    -- Build the new path
    IF NEW.parent_id IS NULL THEN
        NEW.path = formatted_id::ltree;
    ELSE
        SELECT path INTO parent_path FROM filesystem WHERE id = NEW.parent_id;
        NEW.path = parent_path || formatted_id::ltree;
    END IF;

    -- Only cascade updates to descendants if the item was physically MOVED to a new folder
    IF (TG_OP = 'UPDATE') AND (OLD.parent_id IS DISTINCT FROM NEW.parent_id) THEN
        UPDATE filesystem 
        SET path = NEW.path || subpath(path, nlevel(OLD.path))
        WHERE path <@ OLD.path AND id != OLD.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_path
BEFORE INSERT OR UPDATE ON filesystem
FOR EACH ROW EXECUTE FUNCTION update_filesystem_path();


-- 7. INDEXES (Crucial for Performance)

-- GiST index on the ltree column is mandatory for lightning-fast tree queries
CREATE INDEX idx_filesystem_path ON filesystem USING GIST (path);

-- Speed up standard directory listing (e.g., SELECT * WHERE parent_id = '...')
CREATE INDEX idx_filesystem_parent_id ON filesystem (parent_id);

-- Speed up reverse lookups (e.g., "Find all folders containing this image")
CREATE INDEX idx_filesystem_file_id ON filesystem (file_id);

-- Speed up ordered directory listings (active rows only — soft-deleted rows never appear in listings)
CREATE INDEX idx_filesystem_sort ON filesystem (parent_id, sort_order) WHERE deleted_at IS NULL;

-- Prevent duplicate names within a folder, but only among active rows.
-- Soft-deleted rows can keep their original names without blocking a new
-- entry being created with the same name (otherwise you could never re-upload
-- a file you just deleted).
CREATE UNIQUE INDEX unique_name_per_folder
    ON filesystem (parent_id, name) WHERE deleted_at IS NULL;