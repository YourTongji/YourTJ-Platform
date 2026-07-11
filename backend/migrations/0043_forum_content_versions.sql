ALTER TABLE forum.threads
    ADD COLUMN content_version bigint NOT NULL DEFAULT 1,
    ADD CONSTRAINT forum_threads_content_version_positive CHECK (content_version > 0);

ALTER TABLE forum.comments
    ADD COLUMN content_version bigint NOT NULL DEFAULT 1,
    ADD CONSTRAINT forum_comments_content_version_positive CHECK (content_version > 0);

CREATE FUNCTION forum.enforce_content_version_step()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.content_version = OLD.content_version THEN
        NEW.content_version := OLD.content_version + 1;
    ELSIF NEW.content_version <> OLD.content_version + 1 THEN
        RAISE EXCEPTION 'content_version must advance exactly once';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER threads_content_version_step
BEFORE UPDATE OF title, body, content_format ON forum.threads
FOR EACH ROW EXECUTE FUNCTION forum.enforce_content_version_step();

CREATE TRIGGER comments_content_version_step
BEFORE UPDATE OF body, content_format ON forum.comments
FOR EACH ROW EXECUTE FUNCTION forum.enforce_content_version_step();
