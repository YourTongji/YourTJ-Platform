import { create } from "zustand";
import { persist } from "zustand/middleware";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";

export interface ScheduledCourse {
  course: SelectionCourse;
  timeslots: TimeSlot[];
  color: string;
}

interface Conflict {
  withCode: string;
  withName: string;
}

interface ScheduleState {
  staged: ScheduledCourse[];
  addCourse: (course: SelectionCourse, timeslots: TimeSlot[]) => Conflict | null;
  removeCourse: (code: string) => void;
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

function courseCode(course: SelectionCourse) {
  return course.code ?? course.id ?? "";
}

function overlaps(left: TimeSlot, right: TimeSlot) {
  if (left.weekday !== right.weekday) {
    return false;
  }
  const leftStart = left.startSlot ?? 0;
  const leftEnd = left.endSlot ?? leftStart;
  const rightStart = right.startSlot ?? 0;
  const rightEnd = right.endSlot ?? rightStart;
  return leftStart <= rightEnd && rightStart <= leftEnd;
}

export const useScheduleStore = create<ScheduleState>()(
  persist(
    (set, get) => ({
      staged: [],
      addCourse: (course, timeslots) => {
        const code = courseCode(course);
        const existing = get().staged;
        const duplicate = existing.find((item) => courseCode(item.course) === code);
        if (duplicate) {
          return null;
        }
        for (const item of existing) {
          for (const slot of item.timeslots) {
            if (timeslots.some((nextSlot) => overlaps(slot, nextSlot))) {
              return {
                withCode: courseCode(item.course),
                withName: item.course.name ?? courseCode(item.course),
              };
            }
          }
        }
        const color = palette[existing.length % palette.length] ?? palette[0];
        set({ staged: [...existing, { course, timeslots, color }] });
        return null;
      },
      removeCourse: (code) => {
        set({ staged: get().staged.filter((item) => courseCode(item.course) !== code) });
      },
      clear: () => set({ staged: [] }),
    }),
    { name: "yourtj.schedule.v1" },
  ),
);

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
