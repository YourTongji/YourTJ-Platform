-- 0034_social_identity_privacy.sql — profile privacy and public social graph
--
-- Follow is a unilateral public relationship. Mute is private filtering. Block
-- is a bilateral safety boundary and never restores relationships when removed.
--
-- Append-only: never edit an applied migration.

CREATE TABLE identity.profiles (
  account_id       BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  display_name     TEXT CHECK (display_name IS NULL OR char_length(display_name) BETWEEN 1 AND 50),
  bio              TEXT CHECK (bio IS NULL OR char_length(bio) <= 500),
  website          TEXT CHECK (
    website IS NULL OR (
      char_length(website) <= 2048
      AND website ~ '^https://[^[:space:]]+$'
    )
  ),
  avatar_asset_id  BIGINT REFERENCES media.uploads(id) ON DELETE SET NULL,
  banner_asset_id  BIGINT REFERENCES media.uploads(id) ON DELETE SET NULL,
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE identity.profile_privacy (
  account_id            BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  profile_visibility    TEXT NOT NULL DEFAULT 'campus'
    CHECK (profile_visibility IN ('public', 'campus', 'only_me')),
  followers_visibility  TEXT NOT NULL DEFAULT 'followers'
    CHECK (followers_visibility IN ('public', 'campus', 'followers', 'only_me')),
  following_visibility  TEXT NOT NULL DEFAULT 'followers'
    CHECK (following_visibility IN ('public', 'campus', 'followers', 'only_me')),
  discoverable          BOOLEAN NOT NULL DEFAULT TRUE,
  dm_policy             TEXT NOT NULL DEFAULT 'following'
    CHECK (dm_policy IN ('everyone', 'following', 'nobody')),
  updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO identity.profiles (account_id)
SELECT id FROM identity.accounts
ON CONFLICT (account_id) DO NOTHING;

INSERT INTO identity.profile_privacy (account_id)
SELECT id FROM identity.accounts
ON CONFLICT (account_id) DO NOTHING;

-- Arbitrary remote avatar URLs are retired. Controlled media references above
-- are now the only writable profile-image source.
UPDATE identity.accounts SET avatar_url = NULL WHERE avatar_url IS NOT NULL;

CREATE FUNCTION identity.clear_legacy_avatar_url()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  NEW.avatar_url := NULL;
  RETURN NEW;
END;
$$;

CREATE TRIGGER accounts_clear_legacy_avatar_url
BEFORE UPDATE OF avatar_url ON identity.accounts
FOR EACH ROW EXECUTE FUNCTION identity.clear_legacy_avatar_url();

ALTER TABLE identity.accounts
  ADD CONSTRAINT accounts_avatar_url_retired CHECK (avatar_url IS NULL);

-- `user_ignores` keeps its legacy physical name for rolling-deploy compatibility;
-- application and product semantics now treat every row as a block.
CREATE INDEX user_ignores_ignored_account_idx
  ON forum.user_ignores (ignored_account_id, account_id);

CREATE TABLE forum.user_mutes (
  account_id        BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  muted_account_id  BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, muted_account_id),
  CHECK (account_id <> muted_account_id)
);

CREATE INDEX user_mutes_muted_account_idx
  ON forum.user_mutes (muted_account_id, account_id);

CREATE FUNCTION forum.user_pair_blocked(viewer_id BIGINT, other_account_id BIGINT)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
AS $$
  SELECT EXISTS (
    SELECT 1 FROM forum.user_ignores
    WHERE (account_id = viewer_id AND ignored_account_id = other_account_id)
       OR (account_id = other_account_id AND ignored_account_id = viewer_id)
  );
$$;

CREATE FUNCTION forum.user_content_hidden(viewer_id BIGINT, author_id BIGINT)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
AS $$
  SELECT forum.user_pair_blocked(viewer_id, author_id) OR EXISTS (
    SELECT 1 FROM forum.user_mutes
    WHERE account_id = viewer_id AND muted_account_id = author_id
  );
$$;

CREATE TABLE forum.user_follows (
  follower_id  BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  followed_id  BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (follower_id, followed_id),
  CHECK (follower_id <> followed_id)
);

CREATE INDEX user_follows_followed_idx
  ON forum.user_follows (followed_id, follower_id DESC);
CREATE INDEX user_follows_follower_idx
  ON forum.user_follows (follower_id, followed_id DESC);

CREATE TABLE forum.user_social_stats (
  account_id       BIGINT PRIMARY KEY REFERENCES identity.accounts(id) ON DELETE CASCADE,
  follower_count   INTEGER NOT NULL DEFAULT 0 CHECK (follower_count >= 0),
  following_count  INTEGER NOT NULL DEFAULT 0 CHECK (following_count >= 0),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO forum.user_social_stats (account_id, follower_count, following_count)
SELECT account.id,
       (SELECT COUNT(*)::INTEGER FROM forum.user_follows AS incoming
        WHERE incoming.followed_id = account.id),
       (SELECT COUNT(*)::INTEGER FROM forum.user_follows AS outgoing
        WHERE outgoing.follower_id = account.id)
FROM identity.accounts AS account
ON CONFLICT (account_id) DO UPDATE
SET follower_count = EXCLUDED.follower_count,
    following_count = EXCLUDED.following_count,
    updated_at = now();

CREATE FUNCTION forum.apply_user_follow_counts()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    INSERT INTO forum.user_social_stats (account_id)
    SELECT account_id
    FROM (VALUES (NEW.follower_id), (NEW.followed_id)) AS accounts(account_id)
    ORDER BY account_id
    ON CONFLICT (account_id) DO NOTHING;

    PERFORM account_id FROM forum.user_social_stats
    WHERE account_id IN (NEW.follower_id, NEW.followed_id)
    ORDER BY account_id FOR UPDATE;

    UPDATE forum.user_social_stats
    SET follower_count = follower_count + 1, updated_at = now()
    WHERE account_id = NEW.followed_id;

    UPDATE forum.user_social_stats
    SET following_count = following_count + 1, updated_at = now()
    WHERE account_id = NEW.follower_id;
    RETURN NEW;
  END IF;

  PERFORM account_id FROM forum.user_social_stats
  WHERE account_id IN (OLD.follower_id, OLD.followed_id)
  ORDER BY account_id FOR UPDATE;

  UPDATE forum.user_social_stats
  SET follower_count = follower_count - 1, updated_at = now()
  WHERE account_id = OLD.followed_id;

  UPDATE forum.user_social_stats
  SET following_count = following_count - 1, updated_at = now()
  WHERE account_id = OLD.follower_id;
  RETURN OLD;
END;
$$;

CREATE TRIGGER user_follows_apply_counts
AFTER INSERT OR DELETE ON forum.user_follows
FOR EACH ROW EXECUTE FUNCTION forum.apply_user_follow_counts();
