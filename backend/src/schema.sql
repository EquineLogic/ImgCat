create table users (username text primary key, name text not null, password text not null);

create extension "uuid-ossp";
create table sessions (id uuid primary key default uuid_generate_v4(), username text not null references users(username), token text not null, created_at timestamptz default now(), expires_at timestamptz not null);
CREATE EXTENSION ltree;
CREATE TYPE entry_type AS ENUM ('file', 'folder');
CREATE TABLE filesystem (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    type entry_type NOT NULL,
    owner_username TEXT NOT NULL REFERENCES users(username) ON UPDATE CASCADE,
    parent_id UUID REFERENCES filesystem(id) ON DELETE RESTRICT,
    path LTREE,
    
    size_bytes BIGINT DEFAULT 0 CHECK (size_bytes <= 1000000),
    s3_fileid TEXT,
    mime_type TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,

    CONSTRAINT unique_name_per_folder UNIQUE (parent_id, name)
);

CREATE OR REPLACE FUNCTION update_filesystem_path() RETURNS TRIGGER AS $$
DECLARE
    parent_path LTREE;
BEGIN
    -- Find the parent's path
    IF NEW.parent_id IS NULL THEN
        NEW.path = regexp_replace(replace(NEW.name, ' ', '_'), '[^a-zA-Z0-9_]', '', 'g')::ltree;
    ELSE
        SELECT path INTO parent_path FROM filesystem WHERE id = NEW.parent_id;
        NEW.path = parent_path || regexp_replace(replace(NEW.name, ' ', '_'), '[^a-zA-Z0-9_]', '', 'g')::ltree;
    END IF;

    -- If the path actually changed, update all descendants
    IF (TG_OP = 'UPDATE') AND (OLD.path IS DISTINCT FROM NEW.path) THEN
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

ALTER TABLE filesystem
ADD CONSTRAINT check_file_storage_id
CHECK (
    (type = 'folder' AND s3_fileid IS NULL) OR 
    (type = 'file' AND s3_fileid IS NOT NULL AND s3_fileid <> '')
);