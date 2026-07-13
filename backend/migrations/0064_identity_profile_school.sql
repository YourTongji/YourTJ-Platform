-- Public, owner-editable school affiliation for community profiles.
-- Existing and newly created profiles default to the campus this deployment serves.

ALTER TABLE identity.profiles
  ADD COLUMN school TEXT NOT NULL DEFAULT '同济大学';

ALTER TABLE identity.profiles
  ADD CONSTRAINT identity_profiles_school_valid
  CHECK (
    school = btrim(school)
    AND char_length(school) BETWEEN 1 AND 100
    AND school !~ '[[:cntrl:]]'
  );
