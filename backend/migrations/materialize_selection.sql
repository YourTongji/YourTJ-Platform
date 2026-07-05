-- materialize_selection.sql — Materialize selection.* from selection.pk_* raw tables.
-- Idempotent: safe to re-run. Run AFTER step 1 (d1_export → selection.pk_*).
BEGIN;

ALTER SEQUENCE IF EXISTS selection.timeslots_id_seq RESTART;

-- Calendars
INSERT INTO selection.calendars (id, name, is_current)
SELECT calendar_id, calendar_name, false FROM selection.pk_calendars
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;
UPDATE selection.calendars SET is_current = true WHERE id = (SELECT MAX(calendar_id) FROM selection.pk_calendars);

-- Campuses
INSERT INTO selection.campuses (id, name)
SELECT ROW_NUMBER() OVER (ORDER BY campus), campus FROM selection.pk_campuses
ON CONFLICT (id) DO NOTHING;

-- Faculties
INSERT INTO selection.faculties (id, name, campus_id)
SELECT ROW_NUMBER() OVER (ORDER BY faculty), COALESCE(faculty_i18n, faculty), NULL FROM selection.pk_faculties
ON CONFLICT (id) DO NOTHING;

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

INSERT INTO selection.courses (id, code, name, credit, nature_id, calendar_id, campus_id, teacher_name)
SELECT DISTINCT ON (cd.id)
       cd.id,
       COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code),
       COALESCE(NULLIF(TRIM(cd.course_name), ''), NULLIF(TRIM(cd.name), ''), cd.code),
       cd.credit, cd.course_label_id, cd.calendar_id,
       camp.id AS campus_id,
       TRIM(t.teacher_name) AS teacher_name
FROM selection.pk_course_details cd
LEFT JOIN selection.pk_teachers_raw t ON t.teaching_class_id = cd.id
LEFT JOIN selection.campuses camp ON camp.name = cd.campus
ORDER BY cd.id, t.id;

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
