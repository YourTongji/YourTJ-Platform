import { beforeEach, describe, expect, it } from "vitest";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import {
  createScheduleStore,
  findScheduleConflicts,
  parseScheduleImport,
  scheduleStorageKey,
  serializeScheduleExport,
  type ScheduledCourse,
  type ScheduleScope,
  timetableCells,
  timetableMaxSlot,
} from "@/lib/schedule-store";

const scope: ScheduleScope = {
  environment: "/api/v2",
  principal: "account-a",
  calendarId: "122",
};

function course(id: string, scheduleUnknown = false): SelectionCourse {
  return {
    id,
    offeringId: id,
    code: `CS${id}`,
    teachingClassCode: `CS${id}.01`,
    name: `课程 ${id}`,
    credit: 3,
    natureId: null,
    calendarId: "122",
    campusId: null,
    facultyName: null,
    teachingLanguage: null,
    teacherName: null,
    teacherNames: [],
    startWeek: 1,
    endWeek: 16,
    weeksUnknown: false,
    scheduleUnknown,
    status: "unknown",
    catalogueCourseId: null,
  };
}

function slot(
  offeringId: string,
  weekNumbers: number[],
  options: { weeksUnknown?: boolean; startSlot?: number; endSlot?: number } = {},
): TimeSlot {
  const weeksUnknown = options.weeksUnknown ?? false;
  return {
    offeringId,
    courseId: offeringId,
    teacherName: null,
    weekday: 1,
    startSlot: options.startSlot ?? 1,
    endSlot: options.endSlot ?? 2,
    weeks: weeksUnknown ? null : weekNumbers.join(","),
    weekNumbers,
    weeksUnknown,
    location: null,
    locationUnknown: true,
  };
}

function scheduled(id: string, timeslots = [slot(id, [1])]): ScheduledCourse {
  return { course: course(id), timeslots, color: "#007f73" };
}

describe("schedule store", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("uses one persisted key per environment, principal, and calendar", () => {
    expect(scheduleStorageKey(scope)).toBe("yourtj.schedule.v2:%2Fapi%2Fv2:account-a:122");
    expect(scheduleStorageKey(scope)).not.toBe(scheduleStorageKey({ ...scope, principal: "account-b" }));
    expect(scheduleStorageKey(scope)).not.toBe(scheduleStorageKey({ ...scope, calendarId: "123" }));
    expect(scheduleStorageKey(scope)).not.toBe(
      scheduleStorageKey({ ...scope, environment: "https://preview.example/api/v2" }),
    );
  });

  it("migrates only the matching legacy global scope without deleting the source", () => {
    const matchingLegacyScope = JSON.stringify({
      environment: new URL(scope.environment, window.location.origin).toString(),
      principal: scope.principal,
      calendarId: scope.calendarId,
    });
    localStorage.setItem("yourtj.schedule.v2", JSON.stringify({
      state: {
        schedules: {
          [matchingLegacyScope]: [scheduled("1")],
          [JSON.stringify({ ...scope, principal: "account-b" })]: [scheduled("2")],
        },
      },
      version: 2,
    }));

    const store = createScheduleStore(scope);

    expect(store.getState().staged.map((item) => item.course.offeringId)).toEqual(["1"]);
    expect(localStorage.getItem("yourtj.schedule.v2")).not.toBeNull();
    expect(localStorage.getItem(scheduleStorageKey(scope))).toContain('"offeringId":"1"');
  });

  it("upgrades the existing origin-main scoped payload without losing teaching classes", () => {
    localStorage.setItem(scheduleStorageKey(scope), JSON.stringify({
      state: {
        staged: [{
          course: {
            id: "1",
            code: "CS1",
            name: "课程 1",
            credit: 3,
            natureId: null,
            calendarId: "122",
            campusId: null,
            teacherName: null,
            teacherNames: [],
          },
          timeslots: [{
            courseId: "1",
            teacherName: null,
            weekday: 1,
            startSlot: 1,
            endSlot: 2,
            weeks: "1-5单",
            location: "南楼 101",
          }],
          color: "#007f73",
        }],
      },
      version: 2,
    }));

    const staged = createScheduleStore(scope).getState().staged;

    expect(staged[0]?.course.offeringId).toBe("1");
    expect(staged[0]?.course.weeksUnknown).toBe(false);
    expect(staged[0]?.timeslots[0]?.weekNumbers).toEqual([1, 3, 5]);
    expect(staged[0]?.timeslots[0]?.locationUnknown).toBe(false);
  });

  it("never overrides a confirmed conflict", () => {
    const store = createScheduleStore(scope);
    expect(store.getState().addCourse(course("1"), [slot("1", [1, 3])]).status).toBe("added");

    const result = store.getState().addCourse(
      course("2"),
      [slot("2", [3])],
      { allowPossibleConflict: true },
    );

    expect(result.status).toBe("conflict");
    expect(store.getState().staged).toHaveLength(1);
  });

  it("allows an explicit override only for a possible conflict", () => {
    const store = createScheduleStore(scope);
    store.getState().addCourse(course("1"), [slot("1", [1])]);

    const blocked = store.getState().addCourse(
      course("2"),
      [slot("2", [], { weeksUnknown: true })],
    );
    const added = store.getState().addCourse(
      course("2"),
      [slot("2", [], { weeksUnknown: true })],
      { allowPossibleConflict: true },
    );

    expect(blocked.status).toBe("conflict");
    expect(added.status).toBe("added");
    expect(store.getState().staged).toHaveLength(2);
  });

  it("treats a candidate with no trustworthy schedule as a possible conflict", () => {
    const staged = [scheduled("1")];
    expect(findScheduleConflicts(staged, course("2", true), [])[0]?.certainty).toBe("possible");
    expect(findScheduleConflicts(staged, course("2"), [])[0]?.certainty).toBe("possible");
  });

  it("exports no principal and restores only the matching environment and calendar", () => {
    const json = serializeScheduleExport([scheduled("1")], scope);
    expect(json).not.toContain("account-a");
    expect(json).not.toContain("principal");
    expect(parseScheduleImport(json, scope)).toHaveLength(1);
    expect(() => parseScheduleImport(json, { ...scope, calendarId: "123" }))
      .toThrow("文件格式或环境、学期与当前课表不匹配");
    expect(() => parseScheduleImport(json, { ...scope, environment: "https://preview.example/api/v2" }))
      .toThrow("文件格式或环境、学期与当前课表不匹配");
  });

  it("deduplicates offerings and rejects out-of-contract slots", () => {
    const payload = JSON.parse(serializeScheduleExport([scheduled("1"), scheduled("1")], scope));
    expect(parseScheduleImport(JSON.stringify(payload), scope)).toHaveLength(1);

    payload.offerings[0].timeslots[0].endSlot = 21;
    expect(() => parseScheduleImport(JSON.stringify(payload), scope))
      .toThrow("文件中的教学班或时段数据无效");

    const malformedDuplicate = JSON.parse(
      serializeScheduleExport([scheduled("1"), scheduled("1")], scope),
    );
    malformedDuplicate.offerings[1].timeslots[0].endSlot = 21;
    expect(() => parseScheduleImport(JSON.stringify(malformedDuplicate), scope))
      .toThrow("文件中的教学班或时段数据无效");
  });

  it("rejects blank known locations and export files over two MiB", () => {
    const payload = JSON.parse(serializeScheduleExport([scheduled("1")], scope));
    payload.offerings[0].timeslots[0].location = "  ";
    payload.offerings[0].timeslots[0].locationUnknown = false;
    expect(() => parseScheduleImport(JSON.stringify(payload), scope))
      .toThrow("文件中的教学班或时段数据无效");

    const oversized = scheduled("2");
    oversized.course.facultyName = "x".repeat(2 * 1024 * 1024);
    expect(() => serializeScheduleExport([oversized], scope))
      .toThrow("课表 JSON 不能超过 2 MB");
  });

  it("renders observed slots through 20 without exceeding the contract maximum", () => {
    const staged = [scheduled("1", [slot("1", [1], { startSlot: 19, endSlot: 20 })])];
    expect(timetableMaxSlot(staged)).toBe(20);
    expect(timetableCells(staged)["1-20"]).toHaveLength(1);
  });
});
