import { beforeEach, describe, expect, it } from "vitest";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import {
  findScheduleConflicts,
  offeringKey,
  scheduleScopeKey,
  useScheduleStore,
} from "@/lib/schedule-store";

function course(id: string, scheduleUnknown = false): SelectionCourse {
  return {
    id,
    offeringId: id,
    code: "CS101",
    teachingClassCode: `CS101.${id}`,
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
  weeksUnknown = false,
): TimeSlot {
  return {
    offeringId,
    courseId: offeringId,
    teacherName: null,
    weekday: 1,
    startSlot: 1,
    endSlot: 2,
    weeks: weeksUnknown ? null : weekNumbers.join(","),
    weekNumbers,
    weeksUnknown,
    location: null,
    locationUnknown: true,
  };
}

describe("schedule store", () => {
  beforeEach(() => {
    localStorage.clear();
    useScheduleStore.setState({ schedules: {} });
  });

  it("partitions schedules by environment, principal, and calendar", () => {
    expect(scheduleScopeKey("122", "account-a", "/api/v2"))
      .not.toBe(scheduleScopeKey("122", "account-b", "/api/v2"));
    expect(scheduleScopeKey("122", null, "/api/v2"))
      .not.toBe(scheduleScopeKey("121", null, "/api/v2"));
    expect(scheduleScopeKey("122", null, "/api/v2"))
      .not.toBe(scheduleScopeKey("122", null, "https://preview.example/api/v2"));
  });

  it("uses offering identity so parallel classes with one course code remain distinct", () => {
    const scope = scheduleScopeKey("122");
    expect(useScheduleStore.getState().addCourse(scope, course("1"), [slot("1", [1])]).status)
      .toBe("added");
    expect(useScheduleStore.getState().addCourse(scope, course("2"), [slot("2", [2])]).status)
      .toBe("added");
    expect(useScheduleStore.getState().schedules[scope]?.map((item) => offeringKey(item.course)))
      .toEqual(["1", "2"]);
  });

  it("distinguishes confirmed, possible, and disjoint-week conflicts", () => {
    const staged = [{ course: course("1"), timeslots: [slot("1", [1, 3, 5])], color: "#000" }];
    expect(findScheduleConflicts(staged, course("2"), [slot("2", [2, 4, 6])])).toEqual([]);
    expect(findScheduleConflicts(staged, course("2"), [slot("2", [3, 4])])[0]?.certainty)
      .toBe("confirmed");
    expect(findScheduleConflicts(staged, course("2"), [slot("2", [], true)])[0]?.certainty)
      .toBe("possible");
    expect(findScheduleConflicts(staged, course("2", true), [])[0]?.certainty)
      .toBe("possible");
  });
});
