-- 0009_selection_raw_pk.sql — Raw mirror of 一系统 (PK/onesystem) tables.
-- Mirrors the Cloudflare D1 schema exactly, 1:1, as a staging area.
-- All data is refreshed by the sync worker; do NOT edit rows manually.
-- Append-only schema. Tables are used by 0010 to materialize into
-- courses.{courses,teachers,course_aliases} and selection.* normalized tables.

CREATE SCHEMA IF NOT EXISTS selection;

-- Raw calendar (学期历).
CREATE TABLE selection.pk_calendars (
    calendar_id   BIGINT PRIMARY KEY,
    calendar_name TEXT           -- calendarIdI18n
);

-- Raw language dimension (教学语言).
CREATE TABLE selection.pk_languages (
    teaching_language   TEXT PRIMARY KEY,
    teaching_language_i18n TEXT,
    calendar_id          BIGINT REFERENCES selection.pk_calendars(calendar_id)
);

-- Raw course nature (课程性质).
CREATE TABLE selection.pk_course_natures (
    course_label_id   INTEGER PRIMARY KEY,
    course_label_name TEXT,
    calendar_id       BIGINT REFERENCES selection.pk_calendars(calendar_id)
);

-- Per-semester course nature mapping.
CREATE TABLE selection.pk_course_natures_by_calendar (
    calendar_id     BIGINT NOT NULL REFERENCES selection.pk_calendars(calendar_id),
    course_label_id INTEGER NOT NULL,
    course_label_name TEXT,
    PRIMARY KEY (calendar_id, course_label_id)
);

-- Raw assessment mode (考核方式).
CREATE TABLE selection.pk_assessments (
    assessment_mode    TEXT PRIMARY KEY,
    assessment_mode_i18n TEXT,
    calendar_id        BIGINT REFERENCES selection.pk_calendars(calendar_id)
);

-- Raw campus.
CREATE TABLE selection.pk_campuses (
    campus      TEXT PRIMARY KEY,
    campus_i18n TEXT,
    calendar_id BIGINT REFERENCES selection.pk_calendars(calendar_id)
);

-- Raw faculty (开课学院).
CREATE TABLE selection.pk_faculties (
    faculty      TEXT PRIMARY KEY,
    faculty_i18n TEXT,
    calendar_id  BIGINT REFERENCES selection.pk_calendars(calendar_id)
);

-- Raw major (专业).
CREATE TABLE selection.pk_majors (
    id          BIGSERIAL PRIMARY KEY,
    code        TEXT,
    grade       INTEGER,
    name        TEXT UNIQUE,
    calendar_id BIGINT REFERENCES selection.pk_calendars(calendar_id)
);
CREATE INDEX idx_pk_majors_grade ON selection.pk_majors(grade);
CREATE INDEX idx_pk_majors_code  ON selection.pk_majors(code);

-- Raw teaching class detail.
CREATE TABLE selection.pk_course_details (
    id                 BIGINT PRIMARY KEY,    -- teachingClassId
    code               TEXT,                  -- teaching class code
    name               TEXT,
    course_label_id    INTEGER,
    assessment_mode    TEXT,
    period             DOUBLE PRECISION,
    week_hour          DOUBLE PRECISION,
    campus             TEXT,
    number             INTEGER,
    elc_number         INTEGER,
    start_week         INTEGER,
    end_week           INTEGER,
    course_code        TEXT,                  -- canonical course code
    course_name        TEXT,
    credit             DOUBLE PRECISION,
    teaching_language  TEXT,
    faculty            TEXT,
    calendar_id        BIGINT,
    new_course_code    TEXT,
    new_code           TEXT
);
CREATE INDEX idx_pk_course_details_calendar     ON selection.pk_course_details(calendar_id);
CREATE INDEX idx_pk_course_details_course_code  ON selection.pk_course_details(course_code);
CREATE INDEX idx_pk_course_details_code          ON selection.pk_course_details(code);
CREATE INDEX idx_pk_course_details_new_course_code ON selection.pk_course_details(new_course_code);
CREATE INDEX idx_pk_course_details_new_code       ON selection.pk_course_details(new_code);

-- Raw teacher assignment (each teaching class has one or more teachers).
CREATE TABLE selection.pk_teachers_raw (
    id                 BIGINT PRIMARY KEY,
    teaching_class_id  BIGINT REFERENCES selection.pk_course_details(id),
    teacher_code       TEXT,
    teacher_name       TEXT,
    arrange_info_text  TEXT
);
CREATE INDEX idx_pk_teachers_raw_class ON selection.pk_teachers_raw(teaching_class_id);
CREATE INDEX idx_pk_teachers_raw_code  ON selection.pk_teachers_raw(teacher_code);
CREATE INDEX idx_pk_teachers_raw_name  ON selection.pk_teachers_raw(teacher_name);


-- Raw teacher timeslots (排课时间).
CREATE TABLE selection.pk_teacher_timeslots (
    calendar_id       BIGINT NOT NULL,
    teaching_class_id BIGINT NOT NULL,
    occupy_day        INTEGER NOT NULL,
    occupy_section    INTEGER NOT NULL,
    teacher_code      TEXT DEFAULT '',
    teacher_name      TEXT DEFAULT '',
    PRIMARY KEY (calendar_id, teaching_class_id, occupy_day, occupy_section, teacher_code, teacher_name)
);
CREATE INDEX idx_pk_teacher_timeslots_slot  ON selection.pk_teacher_timeslots(calendar_id, occupy_day, occupy_section);
CREATE INDEX idx_pk_teacher_timeslots_class ON selection.pk_teacher_timeslots(teaching_class_id);

-- Course-major binding.
CREATE TABLE selection.pk_major_courses (
    major_id  BIGINT NOT NULL REFERENCES selection.pk_majors(id) ON DELETE CASCADE,
    course_id BIGINT NOT NULL REFERENCES selection.pk_course_details(id) ON DELETE CASCADE,
    PRIMARY KEY (major_id, course_id)
);
CREATE INDEX idx_pk_major_courses_course ON selection.pk_major_courses(course_id);

-- Raw fetch log.
CREATE TABLE selection.pk_fetch_logs (
    fetch_time  BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    msg         TEXT
);
CREATE INDEX idx_pk_fetch_logs_time ON selection.pk_fetch_logs(fetch_time);
