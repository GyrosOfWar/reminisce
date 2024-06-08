CREATE TABLE screenshots (
    "timestamp" TEXT NOT NULL,
    "path" VARCHAR NOT NULL,
    "description" VARCHAR,
    "status" VARCHAR NOT NULL,
    "window_title" VARCHAR NOT NULL,
    "application_name" VARCHAR NOT NULL,
    "text_content" VARCHAR
);