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

export type ScheduleConflictKind = "confirmed" | "possible";

export interface ScheduleConflict {
  kind: ScheduleConflictKind;
  withId: string;
  withName: string;
  existingSlot: TimeSlot;
  candidateSlot: TimeSlot;
}

export type ScheduleAddResult =
  | { status: "added" }
  | { status: "duplicate" }
  | { status: "scopeMismatch" }
  | { status: "conflict"; conflict: ScheduleConflict };

interface ScheduleAddOptions {
  allowPossibleConflict?: boolean;
}

interface ScheduleState {
  staged: ScheduledCourse[];
  addCourse: (
    course: SelectionCourse,
    timeslots: TimeSlot[],
    options?: ScheduleAddOptions,
  ) => ScheduleAddResult;
  removeCourse: (teachingClassId: string) => void;
  clear: () => void;
}

const palette = [
  "#00a08a",
  "#c9a227",
  "#4f87a0",
  "#7ca86e",
  "#b85c38",
  "#8067c6",
  "#c0513a",
  "#3d9970",
];

function teachingClassId(course: SelectionCourse) {
  return course.id;
}

function overlapsBySlot(left: TimeSlot, right: TimeSlot) {
  if (left.weekday !== right.weekday) {
    return false;
  }
  const leftStart = left.startSlot ?? 0;
  const leftEnd = left.endSlot ?? leftStart;
  const rightStart = right.startSlot ?? 0;
  const rightEnd = right.endSlot ?? rightStart;
  return leftStart <= rightEnd && rightStart <= leftEnd;
}

function parseCourseWeeks(value: string | null) {
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
  if (!normalized || normalized.length > 128) {
    return null;
  }

  let globalParity: "单" | "双" | null = null;
  if (normalized.endsWith("单") || normalized.endsWith("双")) {
    globalParity = normalized.at(-1) as "单" | "双";
    normalized = normalized.slice(0, -1);
  }

  const segments = normalized.split(",");
  if (!segments.length || segments.length > 60) {
    return null;
  }
  const weeks = new Set<number>();
  for (let segment of segments) {
    if (!segment) {
      return null;
    }
    let parity = globalParity;
    if (segment.endsWith("单") || segment.endsWith("双")) {
      parity = segment.at(-1) as "单" | "双";
      segment = segment.slice(0, -1);
    }
    const match = /^(\d{1,2})(?:-(\d{1,2}))?$/.exec(segment);
    if (!match) {
      return null;
    }
    const start = Number(match[1]);
    const end = Number(match[2] ?? match[1]);
    if (start < 1 || end > 60 || start > end) {
      return null;
    }
    for (let week = start; week <= end; week += 1) {
      if (parity === "单" && week % 2 === 0) {
        continue;
      }
      if (parity === "双" && week % 2 !== 0) {
        continue;
      }
      weeks.add(week);
    }
  }
  return weeks.size ? weeks : null;
}

function timeslotConflictKind(
  existingSlot: TimeSlot,
  candidateSlot: TimeSlot,
): ScheduleConflictKind | null {
  if (!overlapsBySlot(existingSlot, candidateSlot)) {
    return null;
  }
  const existingWeeks = parseCourseWeeks(existingSlot.weeks);
  const candidateWeeks = parseCourseWeeks(candidateSlot.weeks);
  if (!existingWeeks || !candidateWeeks) {
    return "possible";
  }
  return [...existingWeeks].some((week) => candidateWeeks.has(week))
    ? "confirmed"
    : null;
}

export function scheduleStorageKey(scope: ScheduleScope) {
  const encode = (value: string) => encodeURIComponent(value || "unselected");
  return `yourtj.schedule.v2:${encode(scope.environment)}:${encode(scope.principal)}:${encode(scope.calendarId)}`;
}

function hydrateScheduledCourses(
  persisted: unknown,
  calendarId: string,
): ScheduledCourse[] {
  if (!persisted || typeof persisted !== "object") {
    return [];
  }
  const staged = (persisted as { staged?: unknown }).staged;
  if (!Array.isArray(staged) || staged.length > 100) {
    return [];
  }
  const teachingClassIds = new Set<string>();
  return staged.filter((item): item is ScheduledCourse => {
    if (!item || typeof item !== "object") {
      return false;
    }
    const candidate = item as Partial<ScheduledCourse>;
    const course = candidate.course;
    if (
      !course ||
      typeof course.id !== "string" ||
      !course.id ||
      course.calendarId !== calendarId ||
      !Array.isArray(candidate.timeslots) ||
      typeof candidate.color !== "string" ||
      teachingClassIds.has(course.id)
    ) {
      return false;
    }
    teachingClassIds.add(course.id);
    return true;
  });
}

export function createScheduleStore(scope: ScheduleScope) {
  return createStore<ScheduleState>()(
    persist(
      (set, get) => ({
        staged: [],
        addCourse: (course, timeslots, options) => {
          if (course.calendarId !== scope.calendarId) {
            return { status: "scopeMismatch" };
          }
          const courseId = teachingClassId(course);
          const existing = get().staged;
          const duplicate = existing.find(
            (item) => teachingClassId(item.course) === courseId,
          );
          if (duplicate) {
            return { status: "duplicate" };
          }
          let possibleConflict: ScheduleConflict | null = null;
          for (const item of existing) {
            for (const existingSlot of item.timeslots) {
              for (const candidateSlot of timeslots) {
                const kind = timeslotConflictKind(existingSlot, candidateSlot);
                if (!kind) {
                  continue;
                }
                const conflict: ScheduleConflict = {
                  kind,
                  withId: teachingClassId(item.course),
                  withName: item.course.name ?? teachingClassId(item.course),
                  existingSlot,
                  candidateSlot,
                };
                if (kind === "confirmed") {
                  return { status: "conflict", conflict };
                }
                possibleConflict ??= conflict;
              }
            }
          }
          if (possibleConflict && !options?.allowPossibleConflict) {
            return { status: "conflict", conflict: possibleConflict };
          }
          const color = palette[existing.length % palette.length] ?? palette[0];
          set({ staged: [...existing, { course, timeslots, color }] });
          return { status: "added" };
        },
        removeCourse: (courseId) => {
          set({
            staged: get().staged.filter(
              (item) => teachingClassId(item.course) !== courseId,
            ),
          });
        },
        clear: () => set({ staged: [] }),
      }),
      {
        name: scheduleStorageKey(scope),
        version: 2,
        merge: (persisted, current) => ({
          ...current,
          staged: hydrateScheduledCourses(persisted, scope.calendarId),
        }),
      },
    ),
  );
}

const scheduleStores = new Map<string, StoreApi<ScheduleState>>();

function scheduleStore(scope: ScheduleScope) {
  const key = scheduleStorageKey(scope);
  const existing = scheduleStores.get(key);
  if (existing) {
    return existing;
  }
  const created = createScheduleStore(scope);
  scheduleStores.set(key, created);
  return created;
}

export function useScheduleStore<T>(
  scope: ScheduleScope,
  selector: (state: ScheduleState) => T,
) {
  return useStore(scheduleStore(scope), selector);
}

export function timetableCells(staged: ScheduledCourse[], maxSlot = 13) {
  const cells: Record<string, ScheduledCourse[]> = {};
  for (const item of staged) {
    for (const slot of item.timeslots) {
      const weekday = slot.weekday ?? 0;
      const start = slot.startSlot ?? 0;
      const end = slot.endSlot ?? start;
      for (let section = start; section <= Math.min(end, maxSlot); section += 1) {
        const key = `${weekday}-${section}`;
        cells[key] = [...(cells[key] ?? []), item];
      }
    }
  }
  return cells;
}
