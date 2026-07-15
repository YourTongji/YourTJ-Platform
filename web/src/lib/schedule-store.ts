import { create } from "zustand";
import { persist } from "zustand/middleware";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import { API_BASE_URL } from "@/lib/api/client";

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
  | { status: "conflict"; conflicts: ScheduleConflict[] };

interface ScheduleState {
  schedules: Record<string, ScheduledCourse[]>;
  addCourse: (
    scope: string,
    course: SelectionCourse,
    timeslots: TimeSlot[],
    allowConflicts?: boolean,
  ) => AddCourseResult;
  removeCourse: (scope: string, offeringId: string) => void;
  clear: (scope: string) => void;
}

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

export function scheduleScopeKey(
  calendarId: string,
  accountId?: string | null,
  apiBaseUrl = API_BASE_URL,
) {
  const origin = typeof window === "undefined" ? "server" : window.location.origin;
  return JSON.stringify({
    environment: new URL(apiBaseUrl, origin).toString(),
    principal: accountId || "anonymous",
    calendarId,
  });
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
    if (course.scheduleUnknown || item.course.scheduleUnknown) {
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

export const useScheduleStore = create<ScheduleState>()(
  persist(
    (set, get) => ({
      schedules: {},
      addCourse: (scope, course, timeslots, allowConflicts = false) => {
        const existing = get().schedules[scope] ?? [];
        const offeringId = offeringKey(course);
        if (existing.some((item) => offeringKey(item.course) === offeringId)) {
          return { status: "duplicate", conflicts: [] };
        }
        const conflicts = findScheduleConflicts(existing, course, timeslots);
        if (conflicts.length > 0 && !allowConflicts) {
          return { status: "conflict", conflicts };
        }
        const color = palette[existing.length % palette.length] ?? palette[0];
        set({
          schedules: {
            ...get().schedules,
            [scope]: [...existing, { course, timeslots, color }],
          },
        });
        return { status: "added", conflicts };
      },
      removeCourse: (scope, offeringId) => {
        const existing = get().schedules[scope] ?? [];
        set({
          schedules: {
            ...get().schedules,
            [scope]: existing.filter((item) => offeringKey(item.course) !== offeringId),
          },
        });
      },
      clear: (scope) => {
        const schedules = { ...get().schedules };
        delete schedules[scope];
        set({ schedules });
      },
    }),
    { name: "yourtj.schedule.v2", version: 2 },
  ),
);

export function timetableCells(staged: ScheduledCourse[], maxSlot = 13) {
  const cells: Record<string, ScheduledCourse[]> = {};
  for (const item of staged) {
    for (const slot of item.timeslots) {
      for (let section = slot.startSlot; section <= Math.min(slot.endSlot, maxSlot); section += 1) {
        const key = `${slot.weekday}-${section}`;
        cells[key] = [...(cells[key] ?? []), item];
      }
    }
  }
  return cells;
}
