-- 0013_teacher_names.sql — Add teacher_names array column to selection.courses
-- so multi-teacher teaching classes don't lose teachers to DISTINCT ON.

ALTER TABLE selection.courses ADD COLUMN IF NOT EXISTS teacher_names TEXT[];
