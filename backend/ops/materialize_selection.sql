-- materialize_selection.sql — Materialize selection.* from selection.pk_* raw tables.
-- Idempotent: safe to re-run. Run AFTER step 1 (d1_export → selection.pk_*).
BEGIN;

ALTER SEQUENCE IF EXISTS selection.timeslots_id_seq RESTART;

-- Calendars
-- is_current: uses MAX(calendar_id) as a proxy for "most recent semester."
-- Assumption: higher calendar_id = newer calendar. If upstream ever assigns
-- IDs non-monotonically, switch to the most recent fetch log timestamp instead.
INSERT INTO selection.calendars (id, name, is_current)
SELECT calendar_id, calendar_name, false FROM selection.pk_calendars
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;
UPDATE selection.calendars SET is_current = true WHERE id = (SELECT MAX(calendar_id) FROM selection.pk_calendars);

-- Campuses — natural-key upsert by name; ID is stable via sequence default.
-- New campuses get auto-generated IDs without renumbering existing ones.
INSERT INTO selection.campuses (name)
SELECT DISTINCT campus FROM selection.pk_campuses
ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name;

-- Faculties — natural-key upsert by name; ID is stable via sequence default.
INSERT INTO selection.faculties (name, campus_id)
SELECT DISTINCT COALESCE(faculty_i18n, faculty), NULL FROM selection.pk_faculties
ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name;

-- Majors
INSERT INTO selection.majors (id, name, faculty_id, grade)
SELECT id, name, NULL, grade::TEXT FROM selection.pk_majors
WHERE TRIM(COALESCE(name, '')) != ''
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;

-- Course natures (merge pk_course_natures + pk_course_natures_by_calendar)
INSERT INTO selection.course_natures (id, name)
SELECT course_label_id, course_label_name FROM selection.pk_course_natures
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;
INSERT INTO selection.course_natures (id, name)
SELECT course_label_id, course_label_name FROM selection.pk_course_natures_by_calendar
WHERE NOT EXISTS (SELECT 1 FROM selection.course_natures cn WHERE cn.id = course_label_id)
ON CONFLICT (id) DO NOTHING;

-- Selection courses (one row per teaching class, deduplicated)
DELETE FROM selection.timeslots;
DELETE FROM selection.major_courses;
DELETE FROM selection.courses;

INSERT INTO selection.courses (id, code, name, credit, nature_id, calendar_id, campus_id, teacher_name, teacher_names)
SELECT cd.id,
       COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code),
       COALESCE(NULLIF(TRIM(cd.course_name), ''), NULLIF(TRIM(cd.name), ''), cd.code),
       cd.credit, cd.course_label_id, cd.calendar_id,
       camp.id AS campus_id,
       (SELECT TRIM(t.teacher_name) FROM selection.pk_teachers_raw t
        WHERE t.teaching_class_id = cd.id ORDER BY t.id LIMIT 1) AS teacher_name,
       COALESCE(tn.names, ARRAY[]::TEXT[]) AS teacher_names
FROM selection.pk_course_details cd
LEFT JOIN LATERAL (
    SELECT array_agg(TRIM(t.teacher_name) ORDER BY t.id)
           FILTER (WHERE t.teacher_name IS NOT NULL AND TRIM(t.teacher_name) != '') AS names
    FROM selection.pk_teachers_raw t
    WHERE t.teaching_class_id = cd.id
) tn ON true
LEFT JOIN selection.campuses camp ON camp.name = cd.campus;

-- Major courses
INSERT INTO selection.major_courses (major_id, course_id, grade)
SELECT mc.major_id, mc.course_id, m.grade::TEXT
FROM selection.pk_major_courses mc
LEFT JOIN selection.pk_majors m ON m.id = mc.major_id
WHERE EXISTS (SELECT 1 FROM selection.courses c WHERE c.id = mc.course_id)
ON CONFLICT (major_id, course_id) DO NOTHING;

-- Timeslots (deduplicated)
INSERT INTO selection.timeslots (course_id, teacher_name, weekday, start_slot, end_slot, weeks, location)
SELECT DISTINCT ON (ts.teaching_class_id, ts.occupy_day, ts.occupy_section)
       ts.teaching_class_id, ts.teacher_name, ts.occupy_day,
       (ts.occupy_section * 2 - 1), (ts.occupy_section * 2),
       NULL, NULL
FROM selection.pk_teacher_timeslots ts
WHERE EXISTS (SELECT 1 FROM selection.courses c WHERE c.id = ts.teaching_class_id)
ORDER BY ts.teaching_class_id, ts.occupy_day, ts.occupy_section, ts.teacher_name;

COMMIT;
