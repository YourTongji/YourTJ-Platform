import { useStore } from "zustand";
import { persist } from "zustand/middleware";
import { createStore, type StoreApi } from "zustand/vanilla";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";

export interface ScheduleScope {
  environment: string;
  principal: string;
  calendarId: string;
}

export interface ScheduledCourse {
  course: SelectionCourse;
  timeslots: TimeSlot[];
  color: string;
}

export interface ScheduleConflict {
  withOfferingId: string;
  withCode: string;
  withName: string;
  certainty: "confirmed" | "possible";
}

export type AddCourseResult =
  | { status: "added"; conflicts: ScheduleConflict[] }
  | { status: "duplicate"; conflicts: [] }
  | { status: "scopeMismatch"; conflicts: [] }
  | { status: "conflict"; conflicts: ScheduleConflict[] };

interface AddCourseOptions {
  allowPossibleConflict?: boolean;
}

export interface ScheduleState {
  staged: ScheduledCourse[];
  addCourse: (
    course: SelectionCourse,
    timeslots: TimeSlot[],
    options?: AddCourseOptions,
  ) => AddCourseResult;
  removeCourse: (offeringId: string) => void;
  restore: (courses: ScheduledCourse[]) => void;
  clear: () => void;
}

interface ScheduleExportPayload {
  schema: "yourtj.schedule";
  version: 1;
  scope: {
    environment: string;
    calendarId: string;
  };
  exportedAt: string;
  offerings: Array<{
    course: SelectionCourse;
    timeslots: TimeSlot[];
  }>;
}

const legacyStorageKey = "yourtj.schedule.v2";
const maximumCourses = 100;
const maximumSlot = 20;
const maximumWeek = 30;
const maximumTimeslots = 100;
const maximumExportBytes = 2 * 1024 * 1024;
const palette = [
  "#007f73",
  "#9a7412",
  "#3f7187",
  "#527d46",
  "#a2482d",
  "#6650a4",
  "#9c3c2f",
  "#287455",
];

export function offeringKey(course: SelectionCourse) {
  return course.offeringId || course.id;
}

function normalizeEnvironment(environment: string) {
  if (typeof window === "undefined") {
    return environment;
  }
  return new URL(environment, window.location.origin).toString();
}

export function scheduleStorageKey(scope: ScheduleScope) {
  const encode = (value: string) => encodeURIComponent(value || "unselected");
  return `yourtj.schedule.v2:${encode(scope.environment)}:${encode(scope.principal)}:${encode(scope.calendarId)}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isNullableNumber(value: unknown): value is number | null {
  return value === null || (typeof value === "number" && Number.isFinite(value));
}

function hasValidCourseWeeks(value: Record<string, unknown>) {
  if (value.weeksUnknown === true) {
    return value.startWeek === null && value.endWeek === null;
  }
  return Number.isInteger(value.startWeek)
    && Number(value.startWeek) >= 1
    && Number.isInteger(value.endWeek)
    && Number(value.endWeek) >= Number(value.startWeek)
    && Number(value.endWeek) <= maximumWeek;
}

function isSelectionCourse(value: unknown, calendarId: string): value is SelectionCourse {
  if (!isRecord(value)) return false;
  return typeof value.id === "string"
    && value.id.length > 0
    && typeof value.offeringId === "string"
    && value.offeringId.length > 0
    && value.id === value.offeringId
    && typeof value.code === "string"
    && typeof value.name === "string"
    && value.name.length > 0
    && isNullableString(value.teachingClassCode)
    && isNullableNumber(value.credit)
    && isNullableString(value.natureId)
    && value.calendarId === calendarId
    && isNullableString(value.campusId)
    && isNullableString(value.facultyName)
    && isNullableString(value.teachingLanguage)
    && isNullableString(value.teacherName)
    && (value.teacherName === null || value.teacherName.trim().length > 0)
    && Array.isArray(value.teacherNames)
    && value.teacherNames.every((name) => typeof name === "string")
    && typeof value.weeksUnknown === "boolean"
    && hasValidCourseWeeks(value)
    && typeof value.scheduleUnknown === "boolean"
    && typeof value.status === "string"
    && ["unknown", "active", "cancelled"].includes(value.status)
    && isNullableString(value.catalogueCourseId);
}

function isTimeSlot(value: unknown, offeringId: string): value is TimeSlot {
  if (!isRecord(value)) return false;
  const weekNumbers = value.weekNumbers;
  return value.offeringId === offeringId
    && value.courseId === offeringId
    && isNullableString(value.teacherName)
    && Number.isInteger(value.weekday)
    && Number(value.weekday) >= 1
    && Number(value.weekday) <= 7
    && Number.isInteger(value.startSlot)
    && Number(value.startSlot) >= 1
    && Number(value.startSlot) <= maximumSlot
    && Number.isInteger(value.endSlot)
    && Number(value.endSlot) >= Number(value.startSlot)
    && Number(value.endSlot) <= maximumSlot
    && isNullableString(value.weeks)
    && (
      value.weeksUnknown
        ? value.weeks === null
        : typeof value.weeks === "string" && value.weeks.trim().length > 0
    )
    && Array.isArray(weekNumbers)
    && weekNumbers.length <= maximumWeek
    && weekNumbers.every((week) => Number.isInteger(week) && week >= 1 && week <= maximumWeek)
    && new Set(weekNumbers).size === weekNumbers.length
    && typeof value.weeksUnknown === "boolean"
    && (value.weeksUnknown ? weekNumbers.length === 0 : weekNumbers.length > 0)
    && isNullableString(value.location)
    && typeof value.locationUnknown === "boolean"
    && (
      value.locationUnknown
        ? value.location === null
        : typeof value.location === "string" && value.location.trim().length > 0
    );
}

function parseLegacyWeekNumbers(value: string | null) {
  let normalized = (value ?? "")
    .trim()
    .replace(/\s+/g, "")
    .replaceAll("周", "")
    .replaceAll("至", "-")
    .replaceAll("—", "-")
    .replaceAll("–", "-")
    .replaceAll("~", "-")
    .replaceAll("，", ",")
    .replaceAll("、", ",")
    .replaceAll(";", ",")
    .replaceAll("；", ",");
  if (!normalized || normalized.length > 128) return null;

  let globalParity: "单" | "双" | null = null;
  if (normalized.endsWith("单") || normalized.endsWith("双")) {
    globalParity = normalized.at(-1) as "单" | "双";
    normalized = normalized.slice(0, -1);
  }
  const segments = normalized.split(",");
  if (!segments.length || segments.length > maximumWeek) return null;
  const weeks = new Set<number>();
  for (let segment of segments) {
    if (!segment) return null;
    let parity = globalParity;
    if (segment.endsWith("单") || segment.endsWith("双")) {
      parity = segment.at(-1) as "单" | "双";
      segment = segment.slice(0, -1);
    }
    const match = /^(\d{1,2})(?:-(\d{1,2}))?$/.exec(segment);
    if (!match) return null;
    const start = Number(match[1]);
    const end = Number(match[2] ?? match[1]);
    if (start < 1 || end > maximumWeek || start > end) return null;
    for (let week = start; week <= end; week += 1) {
      if (parity === "单" && week % 2 === 0) continue;
      if (parity === "双" && week % 2 !== 0) continue;
      weeks.add(week);
    }
  }
  return weeks.size > 0 ? [...weeks].sort((left, right) => left - right) : null;
}

function upgradeLegacyTimeSlot(value: unknown, offeringId: string): TimeSlot | null {
  if (!isRecord(value) || value.courseId !== offeringId) return null;
  if (
    !Number.isInteger(value.weekday)
    || Number(value.weekday) < 1
    || Number(value.weekday) > 7
    || !Number.isInteger(value.startSlot)
    || Number(value.startSlot) < 1
    || Number(value.startSlot) > maximumSlot
    || !Number.isInteger(value.endSlot)
    || Number(value.endSlot) < Number(value.startSlot)
    || Number(value.endSlot) > maximumSlot
    || !isNullableString(value.teacherName)
    || (value.teacherName !== null && value.teacherName.trim().length === 0)
    || !isNullableString(value.weeks)
    || !isNullableString(value.location)
  ) {
    return null;
  }
  const weekNumbers = parseLegacyWeekNumbers(value.weeks);
  const location = value.location?.trim() || null;
  return {
    offeringId,
    courseId: offeringId,
    teacherName: value.teacherName,
    weekday: Number(value.weekday),
    startSlot: Number(value.startSlot),
    endSlot: Number(value.endSlot),
    weeks: weekNumbers ? value.weeks : null,
    weekNumbers: weekNumbers ?? [],
    weeksUnknown: weekNumbers === null,
    location,
    locationUnknown: location === null,
  };
}

function upgradeLegacyCourse(
  value: unknown,
  scope: ScheduleScope,
  timeslots: TimeSlot[],
): SelectionCourse | null {
  if (!isRecord(value)) return null;
  if (
    typeof value.id !== "string"
    || !value.id
    || typeof value.code !== "string"
    || typeof value.name !== "string"
    || !value.name
    || !isNullableNumber(value.credit)
    || !isNullableString(value.natureId)
    || value.calendarId !== scope.calendarId
    || !isNullableString(value.campusId)
    || !isNullableString(value.teacherName)
    || !Array.isArray(value.teacherNames)
    || !value.teacherNames.every((name) => typeof name === "string")
  ) {
    return null;
  }
  const knownWeeks = timeslots.flatMap((slot) => slot.weekNumbers);
  const weeksUnknown = timeslots.length === 0 || timeslots.some((slot) => slot.weeksUnknown);
  return {
    id: value.id,
    offeringId: value.id,
    code: value.code,
    teachingClassCode: null,
    name: value.name,
    credit: value.credit,
    natureId: value.natureId,
    calendarId: scope.calendarId,
    campusId: value.campusId,
    facultyName: null,
    teachingLanguage: null,
    teacherName: value.teacherName,
    teacherNames: value.teacherNames,
    startWeek: weeksUnknown ? null : Math.min(...knownWeeks),
    endWeek: weeksUnknown ? null : Math.max(...knownWeeks),
    weeksUnknown,
    scheduleUnknown: timeslots.length === 0,
    status: "unknown",
    catalogueCourseId: null,
  };
}

function validateScheduledCourses(
  value: unknown,
  scope: ScheduleScope,
  allowLegacy = false,
): ScheduledCourse[] | null {
  if (!Array.isArray(value) || value.length > maximumCourses) return null;
  const offeringIds = new Set<string>();
  const result: ScheduledCourse[] = [];
  for (const item of value) {
    if (!isRecord(item) || !isRecord(item.course)) return null;
    const rawOfferingId = typeof item.course.offeringId === "string"
      ? item.course.offeringId
      : item.course.id;
    if (typeof rawOfferingId !== "string" || !rawOfferingId) return null;
    if (!Array.isArray(item.timeslots) || item.timeslots.length > maximumTimeslots) return null;
    const timeslots = item.timeslots.map((slot) => {
      if (isTimeSlot(slot, rawOfferingId)) return slot;
      return allowLegacy ? upgradeLegacyTimeSlot(slot, rawOfferingId) : null;
    });
    if (timeslots.some((slot) => slot === null)) return null;
    const normalizedTimeslots = timeslots as TimeSlot[];
    const normalizedCourse = isSelectionCourse(item.course, scope.calendarId)
      ? item.course
      : allowLegacy
        ? upgradeLegacyCourse(item.course, scope, normalizedTimeslots)
        : null;
    if (!normalizedCourse) return null;
    const offeringId = offeringKey(normalizedCourse);
    if (offeringIds.has(offeringId)) continue;
    const color = typeof item.color === "string" && item.color.length <= 32
      ? item.color
      : palette[result.length % palette.length] ?? palette[0];
    offeringIds.add(offeringId);
    result.push({ course: normalizedCourse, timeslots: normalizedTimeslots, color });
  }
  return result;
}

function hydrateScheduledCourses(persisted: unknown, scope: ScheduleScope) {
  if (!isRecord(persisted)) return [];
  return validateScheduledCourses(persisted.staged, scope, true) ?? [];
}

function matchesLegacyScope(value: string, scope: ScheduleScope) {
  try {
    const candidate = JSON.parse(value) as unknown;
    return isRecord(candidate)
      && candidate.calendarId === scope.calendarId
      && candidate.principal === scope.principal
      && typeof candidate.environment === "string"
      && normalizeEnvironment(candidate.environment) === normalizeEnvironment(scope.environment);
  } catch {
    return false;
  }
}

function migrateLegacySchedule(scope: ScheduleScope) {
  if (typeof window === "undefined") return;
  const targetKey = scheduleStorageKey(scope);
  if (window.localStorage.getItem(targetKey)) return;
  const legacyValue = window.localStorage.getItem(legacyStorageKey);
  if (!legacyValue) return;
  try {
    const parsed = JSON.parse(legacyValue) as unknown;
    if (!isRecord(parsed) || !isRecord(parsed.state) || !isRecord(parsed.state.schedules)) return;
    const entry = Object.entries(parsed.state.schedules)
      .find(([key]) => matchesLegacyScope(key, scope));
    if (!entry) return;
    const staged = validateScheduledCourses(entry[1], scope, true);
    if (!staged) return;
    window.localStorage.setItem(targetKey, JSON.stringify({ state: { staged }, version: 2 }));
  } catch {
    // Invalid legacy data is ignored so it cannot replace a valid scoped schedule.
  }
}

function occupiedAtSameTime(left: TimeSlot, right: TimeSlot) {
  return left.weekday === right.weekday
    && left.startSlot <= right.endSlot
    && right.startSlot <= left.endSlot;
}

function slotConflict(left: TimeSlot, right: TimeSlot): ScheduleConflict["certainty"] | null {
  if (!occupiedAtSameTime(left, right)) return null;
  if (left.weeksUnknown || right.weeksUnknown) return "possible";
  if (left.weekNumbers.length === 0 || right.weekNumbers.length === 0) return "possible";
  const leftWeeks = new Set(left.weekNumbers);
  return right.weekNumbers.some((week) => leftWeeks.has(week)) ? "confirmed" : null;
}

export function findScheduleConflicts(
  staged: ScheduledCourse[],
  course: SelectionCourse,
  timeslots: TimeSlot[],
) {
  const conflicts: ScheduleConflict[] = [];
  for (const item of staged) {
    let certainty: ScheduleConflict["certainty"] | null = null;
    if (
      course.scheduleUnknown
      || timeslots.length === 0
      || item.course.scheduleUnknown
      || item.timeslots.length === 0
    ) {
      certainty = "possible";
    } else {
      for (const existing of item.timeslots) {
        for (const candidate of timeslots) {
          const next = slotConflict(existing, candidate);
          if (next === "confirmed") {
            certainty = "confirmed";
            break;
          }
          if (next === "possible") certainty = "possible";
        }
        if (certainty === "confirmed") break;
      }
    }
    if (certainty) {
      conflicts.push({
        withOfferingId: offeringKey(item.course),
        withCode: item.course.code,
        withName: item.course.name,
        certainty,
      });
    }
  }
  return conflicts;
}

export function createScheduleStore(scope: ScheduleScope) {
  migrateLegacySchedule(scope);
  return createStore<ScheduleState>()(
    persist(
      (set, get) => ({
        staged: [],
        addCourse: (course, timeslots, options) => {
          if (course.calendarId !== scope.calendarId) {
            return { status: "scopeMismatch", conflicts: [] };
          }
          const existing = get().staged;
          const offeringId = offeringKey(course);
          if (existing.some((item) => offeringKey(item.course) === offeringId)) {
            return { status: "duplicate", conflicts: [] };
          }
          const conflicts = findScheduleConflicts(existing, course, timeslots);
          const hasConfirmedConflict = conflicts.some(({ certainty }) => certainty === "confirmed");
          if (
            hasConfirmedConflict
            || (conflicts.length > 0 && !options?.allowPossibleConflict)
          ) {
            return { status: "conflict", conflicts };
          }
          const color = palette[existing.length % palette.length] ?? palette[0];
          set({ staged: [...existing, { course, timeslots, color }] });
          return { status: "added", conflicts };
        },
        removeCourse: (offeringId) => {
          set({ staged: get().staged.filter((item) => offeringKey(item.course) !== offeringId) });
        },
        restore: (courses) => set({ staged: courses }),
        clear: () => set({ staged: [] }),
      }),
      {
        name: scheduleStorageKey(scope),
        version: 2,
        merge: (persisted, current) => ({
          ...current,
          staged: hydrateScheduledCourses(persisted, scope),
        }),
      },
    ),
  );
}

const scheduleStores = new Map<string, StoreApi<ScheduleState>>();

export function getScheduleStore(scope: ScheduleScope) {
  const key = scheduleStorageKey(scope);
  const existing = scheduleStores.get(key);
  if (existing) return existing;
  const created = createScheduleStore(scope);
  scheduleStores.set(key, created);
  return created;
}

export function useScheduleStore<T>(
  scope: ScheduleScope,
  selector: (state: ScheduleState) => T,
) {
  return useStore(getScheduleStore(scope), selector);
}

export function serializeScheduleExport(staged: ScheduledCourse[], scope: ScheduleScope) {
  const payload: ScheduleExportPayload = {
    schema: "yourtj.schedule",
    version: 1,
    scope: {
      environment: normalizeEnvironment(scope.environment),
      calendarId: scope.calendarId,
    },
    exportedAt: new Date().toISOString(),
    offerings: staged.map(({ course, timeslots }) => ({ course, timeslots })),
  };
  const serialized = JSON.stringify(payload, null, 2);
  if (new TextEncoder().encode(serialized).byteLength > maximumExportBytes) {
    throw new Error("课表 JSON 不能超过 2 MB");
  }
  return serialized;
}

export function parseScheduleImport(value: string, scope: ScheduleScope) {
  if (new TextEncoder().encode(value).byteLength > maximumExportBytes) {
    throw new Error("课表 JSON 不能超过 2 MB");
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(value) as unknown;
  } catch {
    throw new Error("文件不是有效的 JSON");
  }
  if (
    !isRecord(parsed)
    || parsed.schema !== "yourtj.schedule"
    || parsed.version !== 1
    || !isRecord(parsed.scope)
    || parsed.scope.calendarId !== scope.calendarId
    || typeof parsed.scope.environment !== "string"
    || normalizeEnvironment(parsed.scope.environment) !== normalizeEnvironment(scope.environment)
    || !Array.isArray(parsed.offerings)
  ) {
    throw new Error("文件格式或环境、学期与当前课表不匹配");
  }
  const candidates = parsed.offerings.map((item, index) => {
    if (!isRecord(item)) return item;
    return {
      course: item.course,
      timeslots: item.timeslots,
      color: palette[index % palette.length] ?? palette[0],
    };
  });
  const courses = validateScheduledCourses(candidates, scope);
  if (!courses) throw new Error("文件中的教学班或时段数据无效");
  return courses;
}

export function timetableMaxSlot(staged: ScheduledCourse[]) {
  const observed = staged.reduce(
    (highest, item) => item.timeslots.reduce(
      (courseHighest, slot) => Math.max(courseHighest, slot.endSlot),
      highest,
    ),
    13,
  );
  return Math.min(maximumSlot, Math.max(13, observed));
}

export function timetableCells(staged: ScheduledCourse[], maxSlot = timetableMaxSlot(staged)) {
  const cells: Record<string, ScheduledCourse[]> = {};
  for (const item of staged) {
    for (const slot of item.timeslots) {
      for (
        let section = Math.max(1, slot.startSlot);
        section <= Math.min(slot.endSlot, maxSlot, maximumSlot);
        section += 1
      ) {
        const key = `${slot.weekday}-${section}`;
        cells[key] = [...(cells[key] ?? []), item];
      }
    }
  }
  return cells;
}
