-- Persist an explicit source format so legacy plain text is never reinterpreted as Markdown.

ALTER TABLE forum.threads
    ADD COLUMN content_format text NOT NULL DEFAULT 'plain_v1',
    ADD CONSTRAINT threads_content_format_valid
        CHECK (content_format IN ('plain_v1', 'markdown_v1'));

ALTER TABLE forum.comments
    ADD COLUMN content_format text NOT NULL DEFAULT 'plain_v1',
    ADD CONSTRAINT comments_content_format_valid
        CHECK (content_format IN ('plain_v1', 'markdown_v1'));

ALTER TABLE forum.post_revisions
    ADD COLUMN old_content_format text NOT NULL DEFAULT 'plain_v1',
    ADD CONSTRAINT post_revisions_content_format_valid
        CHECK (old_content_format IN ('plain_v1', 'markdown_v1'));
