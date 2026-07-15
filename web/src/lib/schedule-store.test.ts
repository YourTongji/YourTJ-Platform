import { beforeEach, describe, expect, it } from "vitest";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";

import {
  createScheduleStore,
  scheduleStorageKey,
  type ScheduleScope,
} from "./schedule-store";

function teachingClass(
  id: string,
  calendarId: string,
  code = "CS101",
): SelectionCourse {
  return {
    id,
    code,
    name: `教学班 ${id}`,
    credit: 2,
    natureId: null,
    calendarId,
    campusId: null,
    teacherName: null,
    teacherNames: [],
  };
}

function timeslot(
  teachingClassId: string,
  weeks: string | null = null,
): TimeSlot {
  return {
    courseId: teachingClassId,
    teacherName: null,
    weekday: 1,
    startSlot: 1,
    endSlot: 2,
    weeks,
    location: null,
  };
}

describe("schedule storage scopes", () => {
  beforeEach(() => localStorage.clear());

  it("keeps account schedules in separate persisted scopes", () => {
    const accountOne: ScheduleScope = {
      environment: "/api/v2",
      principal: "account-1",
      calendarId: "calendar-1",
    };
    const accountTwo: ScheduleScope = {
      ...accountOne,
      principal: "account-2",
    };
    const firstStore = createScheduleStore(accountOne);
    const secondStore = createScheduleStore(accountTwo);

    firstStore
      .getState()
      .addCourse(teachingClass("teaching-class-1", "calendar-1"), [
        timeslot("teaching-class-1"),
      ]);

    expect(secondStore.getState().staged).toEqual([]);
    expect(createScheduleStore(accountOne).getState().staged).toHaveLength(1);
    expect(scheduleStorageKey(accountOne)).not.toBe(scheduleStorageKey(accountTwo));
  });

  it("restores each semester without sharing same-code teaching classes", () => {
    const firstSemester: ScheduleScope = {
      environment: "https://preview.example/api/v2",
      principal: "account-1",
      calendarId: "calendar-1",
    };
    const secondSemester: ScheduleScope = {
      ...firstSemester,
      calendarId: "calendar-2",
    };
    const firstStore = createScheduleStore(firstSemester);
    const secondStore = createScheduleStore(secondSemester);

    firstStore
      .getState()
      .addCourse(teachingClass("teaching-class-1", "calendar-1"), []);
    secondStore
      .getState()
      .addCourse(teachingClass("teaching-class-2", "calendar-2"), []);

    expect(createScheduleStore(firstSemester).getState().staged[0]?.course.id).toBe(
      "teaching-class-1",
    );
    expect(createScheduleStore(secondSemester).getState().staged[0]?.course.id).toBe(
      "teaching-class-2",
    );
    expect(scheduleStorageKey(firstSemester)).not.toBe(
      scheduleStorageKey(secondSemester),
    );
  });

  it("drops persisted teaching classes that do not belong to the storage calendar", () => {
    const scope: ScheduleScope = {
      environment: "/api/v2",
      principal: "account-1",
      calendarId: "calendar-1",
    };
    localStorage.setItem(
      scheduleStorageKey(scope),
      JSON.stringify({
        state: {
          staged: [
            {
              course: teachingClass("class-2", "calendar-2"),
              timeslots: [],
              color: "#00a08a",
            },
          ],
        },
        version: 2,
      }),
    );

    expect(createScheduleStore(scope).getState().staged).toEqual([]);
  });
});

describe("schedule conflicts", () => {
  const scope: ScheduleScope = {
    environment: "/api/v2",
    principal: "account-1",
    calendarId: "calendar-1",
  };

  beforeEach(() => localStorage.clear());

  it("blocks only a confirmed intersection of parseable weeks", () => {
    const store = createScheduleStore(scope);
    store
      .getState()
      .addCourse(teachingClass("class-1", "calendar-1"), [
        timeslot("class-1", "1-16单"),
      ]);

    const disjoint = store
      .getState()
      .addCourse(teachingClass("class-2", "calendar-1"), [
        timeslot("class-2", "2-16双"),
      ]);
    const confirmed = store
      .getState()
      .addCourse(teachingClass("class-3", "calendar-1"), [
        timeslot("class-3", "3-5"),
      ]);

    expect(disjoint).toEqual({ status: "added" });
    expect(confirmed).toMatchObject({
      status: "conflict",
      conflict: { kind: "confirmed", withId: "class-1" },
    });
    expect(store.getState().staged.map((item) => item.course.id)).toEqual([
      "class-1",
      "class-2",
    ]);
  });

  it.each([null, "未知周次"])(
    "treats missing or unparseable weeks %s as possible and permits an override",
    (candidateWeeks) => {
      const store = createScheduleStore(scope);
      store
        .getState()
        .addCourse(teachingClass("class-1", "calendar-1"), [
          timeslot("class-1", "1-16"),
        ]);

      const possible = store
        .getState()
        .addCourse(teachingClass("class-2", "calendar-1"), [
          timeslot("class-2", candidateWeeks),
        ]);
      const overridden = store
        .getState()
        .addCourse(
          teachingClass("class-2", "calendar-1"),
          [timeslot("class-2", candidateWeeks)],
          { allowPossibleConflict: true },
        );

      expect(possible).toMatchObject({
        status: "conflict",
        conflict: { kind: "possible", withId: "class-1" },
      });
      expect(overridden).toEqual({ status: "added" });
      expect(store.getState().staged).toHaveLength(2);
    },
  );

  it("keeps repeated teaching-class additions idempotent", () => {
    const store = createScheduleStore(scope);
    const course = teachingClass("class-1", "calendar-1");

    expect(store.getState().addCourse(course, [])).toEqual({ status: "added" });
    expect(store.getState().addCourse(course, [])).toEqual({
      status: "duplicate",
    });
    expect(store.getState().staged).toHaveLength(1);
  });

  it.each([null, "calendar-2"])(
    "rejects a teaching class from a different or unknown calendar (%s)",
    (calendarId) => {
      const store = createScheduleStore(scope);
      const course = teachingClass("class-1", "calendar-1");
      course.calendarId = calendarId;

      expect(store.getState().addCourse(course, [])).toEqual({
        status: "scopeMismatch",
      });
      expect(store.getState().staged).toEqual([]);
    },
  );
});
