CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    external_id TEXT,
    source TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    assignee TEXT,
    labels TEXT,
    due_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    custom_fields TEXT
);
