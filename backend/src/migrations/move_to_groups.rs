use crate::migrations::Migration;

pub static MIGRATION: Migration = Migration {
    id: "move_to_groups",
    description: "Add support for group users",
    up: |pool| {
        Box::pin(async move {
            let mut tx = pool.begin().await?;

            let stmts = [
                "DROP VIEW accessible_filesystem", // unused
                "ALTER TABLE users ADD COLUMN user_type TEXT NOT NULL DEFAULT 'user' CHECK (user_type IN ('user', 'group'))",
                "CREATE UNIQUE INDEX users_id_type_unique ON users(id, user_type)",
                "ALTER TABLE users ALTER COLUMN password DROP NOT NULL",
                "ALTER TABLE users ADD CONSTRAINT check_user_has_password CHECK (
                    (user_type = 'group' AND password IS NULL) 
                    OR (user_type = 'user' AND password IS NOT NULL)
                )",
                "CREATE TYPE group_member_state AS ENUM ('pending_invite', 'accepted')",
                "CREATE TABLE group_members (
                    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                    
                    --- Source group
                    group_id UUID NOT NULL,
                    group_type TEXT NOT NULL DEFAULT 'group' CHECK (group_type = 'group'),
                    
                    --- Target user
                    user_id UUID NOT NULL,
                    user_type TEXT NOT NULL DEFAULT 'user' CHECK (user_type = 'user'),
                    
                    --- Sender
                    sender_id UUID NOT NULL,
                    sender_type TEXT NOT NULL DEFAULT 'user' CHECK (user_type = 'user'),

                    -- metadata
                    perms TEXT[] NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    state group_member_state NOT NULL DEFAULT 'pending_invite', 
                    -- fkeys
                    FOREIGN KEY (group_id, group_type) REFERENCES users(id, user_type),
                    FOREIGN KEY (user_id, user_type) REFERENCES users(id, user_type),
                    FOREIGN KEY (sender_id, sender_type) REFERENCES users(id, user_type),
                    -- Avoid duplcicate invites
                    UNIQUE(group_id, user_id)
                )",
                "DROP INDEX idx_share_requests_recipient",
                "DROP TABLE share_requests",
                "DROP INDEX idx_permissions_grantee",
                "DROP INDEX idx_permissions_filesystem",
                "DROP TABLE permissions",
                "ALTER TABLE sessions ADD COLUMN active_membership_id UUID REFERENCES group_members(id) ON DELETE CASCADE"
            ];

            for stmt in stmts.iter() {
                sqlx::query(stmt)
                    .execute(&mut *tx)
                    .await?;
            }

            tx.commit().await?;

            Ok(())
        })
    },
};
