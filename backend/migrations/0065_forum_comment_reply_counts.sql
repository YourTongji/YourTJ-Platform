-- Write-maintained count of directly nested, currently visible replies.

ALTER TABLE forum.comments
  ADD COLUMN reply_count INT NOT NULL DEFAULT 0 CHECK (reply_count >= 0);

UPDATE forum.comments AS parent
SET reply_count = (
  SELECT count(*)::INT
  FROM forum.comments AS child
  WHERE child.parent_id = parent.id
    AND child.deleted_at IS NULL
    AND child.hidden_at IS NULL
);

CREATE OR REPLACE FUNCTION forum.refresh_comment_reply_count()
RETURNS TRIGGER AS $$
DECLARE
  old_parent_id BIGINT;
  new_parent_id BIGINT;
  old_is_visible BOOLEAN;
  new_is_visible BOOLEAN;
BEGIN
  old_parent_id := CASE WHEN TG_OP IN ('UPDATE', 'DELETE') THEN OLD.parent_id ELSE NULL END;
  new_parent_id := CASE WHEN TG_OP IN ('INSERT', 'UPDATE') THEN NEW.parent_id ELSE NULL END;
  old_is_visible := TG_OP IN ('UPDATE', 'DELETE')
    AND OLD.parent_id IS NOT NULL
    AND OLD.deleted_at IS NULL
    AND OLD.hidden_at IS NULL;
  new_is_visible := TG_OP IN ('INSERT', 'UPDATE')
    AND NEW.parent_id IS NOT NULL
    AND NEW.deleted_at IS NULL
    AND NEW.hidden_at IS NULL;

  IF old_is_visible AND (NOT new_is_visible OR new_parent_id IS DISTINCT FROM old_parent_id) THEN
    UPDATE forum.comments SET reply_count = reply_count - 1 WHERE id = old_parent_id;
  END IF;

  IF new_is_visible AND (NOT old_is_visible OR new_parent_id IS DISTINCT FROM old_parent_id) THEN
    UPDATE forum.comments SET reply_count = reply_count + 1 WHERE id = new_parent_id;
  END IF;

  IF TG_OP = 'DELETE' THEN
    RETURN OLD;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER forum_comments_reply_count_refresh
AFTER INSERT OR DELETE OR UPDATE OF parent_id, deleted_at, hidden_at
ON forum.comments
FOR EACH ROW EXECUTE FUNCTION forum.refresh_comment_reply_count();
