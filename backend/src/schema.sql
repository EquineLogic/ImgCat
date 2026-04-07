-- 1. EXTENSIONS
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "ltree";

-- 2. AUTHENTICATION TABLES (Users & Sessions)
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    password TEXT NOT NULL,
    trash_retention_days INTEGER NOT NULL DEFAULT 30
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

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
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

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

-- Speed up owner-based queries
CREATE INDEX idx_filesystem_owner ON filesystem (owner_id);

-- Speed up ordered directory listings (active rows only — soft-deleted rows never appear in listings)
CREATE INDEX idx_filesystem_sort ON filesystem (parent_id, sort_order) WHERE deleted_at IS NULL;

-- Prevent duplicate names within a folder, but only among active rows.
-- Soft-deleted rows can keep their original names without blocking a new
-- entry being created with the same name (otherwise you could never re-upload
-- a file you just deleted).
CREATE UNIQUE INDEX unique_name_per_folder
    ON filesystem (parent_id, name)
    NULLS NOT DISTINCT
    WHERE deleted_at IS NULL;


-- 8. SHARING & PERMISSIONS

-- Permission levels: viewer (read-only), editor (future: mutate), owner (future: full control)
CREATE TYPE access_level AS ENUM ('viewer', 'editor', 'owner');

-- Transient inbox for pending share requests. Rows are deleted on accept or decline.
CREATE TABLE share_requests (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    filesystem_id UUID NOT NULL REFERENCES filesystem(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_level access_level NOT NULL DEFAULT 'viewer',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(filesystem_id, recipient_id)
);

CREATE INDEX idx_share_requests_recipient ON share_requests(recipient_id);

-- Source of truth for granted access. Only contains accepted permissions.
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    filesystem_id UUID NOT NULL REFERENCES filesystem(id) ON DELETE CASCADE,
    grantee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_level access_level NOT NULL DEFAULT 'viewer',
    granted_by_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(filesystem_id, grantee_id)
);

CREATE INDEX idx_permissions_grantee ON permissions(grantee_id);
CREATE INDEX idx_permissions_filesystem ON permissions(filesystem_id);

-- 9. SECURITY VIEW
-- Merges "I own it" with "I have permission" so backend queries can use
-- `WHERE accessible_by = $user_id` instead of duplicating ownership/permission logic.
CREATE VIEW accessible_filesystem AS
-- Items the user owns
SELECT fs.id, fs.parent_id, fs.name, fs.type, fs.file_id,
       fs.owner_id AS accessible_by,
       'owner'::access_level AS access_level,
       fs.path, fs.sort_order, fs.created_at, fs.updated_at
FROM filesystem fs
WHERE fs.deleted_at IS NULL

UNION ALL

-- Items shared via permissions (includes descendants of shared folders via LTREE)
SELECT fs.id, fs.parent_id, fs.name, fs.type, fs.file_id,
       p.grantee_id AS accessible_by,
       p.access_level,
       fs.path, fs.sort_order, fs.created_at, fs.updated_at
FROM filesystem fs
JOIN permissions p ON fs.path <@ (
    SELECT pfs.path FROM filesystem pfs
    WHERE pfs.id = p.filesystem_id AND pfs.deleted_at IS NULL
)
WHERE fs.deleted_at IS NULL;
