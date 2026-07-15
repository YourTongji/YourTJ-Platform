import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import { scheduleScopeKey, useScheduleStore } from "@/lib/schedule-store";
import { expectNoAccessibilityViolations } from "@/test/accessibility";
import { SchedulePage } from "./schedule-page";

const apiMocks = vi.hoisted(() => ({
  calendars: vi.fn(),
  latest: vi.fn(),
  grades: vi.fn(),
  majors: vi.fn(),
  natures: vi.fn(),
  offerings: vi.fn(),
  timeslots: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ account: null }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    calendars: apiMocks.calendars,
    selectionLatestUpdate: apiMocks.latest,
    grades: apiMocks.grades,
    majors: apiMocks.majors,
    courseNatures: apiMocks.natures,
    selectionOfferings: apiMocks.offerings,
    selectionOfferingTimeslots: apiMocks.timeslots,
  },
}));

function course(id: string, code = "CS101"): SelectionCourse {
  return {
    id,
    offeringId: id,
    code,
    teachingClassCode: `${code}.01`,
    name: code === "CS101" ? "数据结构" : "离散数学",
    credit: 3,
    natureId: null,
    calendarId: "122",
    campusId: null,
    facultyName: "电子与信息工程学院",
    teachingLanguage: "中文",
    teacherName: "张老师",
    teacherNames: ["张老师"],
    startWeek: 1,
    endWeek: 16,
    weeksUnknown: false,
    scheduleUnknown: false,
    status: "unknown",
    catalogueCourseId: null,
  };
}

function slot(offeringId: string, weeksUnknown = false): TimeSlot {
  return {
    offeringId,
    courseId: offeringId,
    teacherName: "张老师",
    weekday: 1,
    startSlot: 1,
    endSlot: 2,
    weeks: weeksUnknown ? null : "1",
    weekNumbers: weeksUnknown ? [] : [1],
    weeksUnknown,
    location: "南楼 101",
    locationUnknown: false,
  };
}

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/schedule?courseCode=CS101"]}>
        <SchedulePage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("SchedulePage", () => {
  beforeEach(() => {
    localStorage.clear();
    useScheduleStore.setState({ schedules: {} });
    apiMocks.calendars.mockReset().mockResolvedValue([{ id: "122", name: "2026-2027-1", isCurrent: true }]);
    apiMocks.latest.mockReset().mockResolvedValue({ updatedAt: null, importedAt: null, stale: true, staleAfterHours: 168 });
    apiMocks.grades.mockReset().mockResolvedValue([]);
    apiMocks.majors.mockReset().mockResolvedValue([]);
    apiMocks.natures.mockReset().mockResolvedValue([]);
    apiMocks.offerings.mockReset().mockResolvedValue({ items: [course("9001")], nextCursor: null, hasMore: false });
    apiMocks.timeslots.mockReset().mockResolvedValue([slot("9001")]);
  });

  it("opens a course-code deep link at offering level and stores the teaching class locally", async () => {
    const user = userEvent.setup();
    const view = renderPage();
    await user.click(await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" }));

    await waitFor(() => expect(apiMocks.timeslots).toHaveBeenCalledWith("9001", expect.any(AbortSignal)));
    const scope = scheduleScopeKey("122");
    expect(useScheduleStore.getState().schedules[scope]?.[0]?.course.offeringId).toBe("9001");
    await expectNoAccessibilityViolations(view.container);
  });

  it("requires an explicit decision before adding a confirmed conflict", async () => {
    const scope = scheduleScopeKey("122");
    useScheduleStore.getState().addCourse(scope, course("8001", "MATH101"), [slot("8001")]);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" }));
    expect(await screen.findByText("发现课表冲突")).toBeVisible();
    expect(screen.getByText("已确认冲突")).toBeVisible();
    expect(useScheduleStore.getState().schedules[scope]).toHaveLength(1);

    await user.click(screen.getByRole("button", { name: "仍然加入并标记" }));
    expect(useScheduleStore.getState().schedules[scope]).toHaveLength(2);
  });

  it("cancels an older timeslot lookup when another teaching class is selected", async () => {
    apiMocks.offerings.mockResolvedValue({
      items: [course("9001"), course("9002", "MATH101")],
      nextCursor: null,
      hasMore: false,
    });
    let firstSignal: AbortSignal | undefined;
    apiMocks.timeslots.mockImplementation((offeringId: string, signal: AbortSignal) => {
      if (offeringId !== "9001") return Promise.resolve([slot(offeringId)]);
      firstSignal = signal;
      return new Promise<TimeSlot[]>((_, reject) => {
        signal.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")));
      });
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" }));
    await waitFor(() => expect(firstSignal).toBeDefined());
    await user.click(screen.getByRole("button", { name: "加入离散数学教学班MATH101.01" }));

    await waitFor(() => expect(firstSignal?.aborted).toBe(true));
    await waitFor(() => expect(
      useScheduleStore.getState().schedules[scheduleScopeKey("122")]?.[0]?.course.offeringId,
    ).toBe("9002"));
  });

  it("aborts an in-flight add and ignores its response after the semester changes", async () => {
    apiMocks.calendars.mockResolvedValue([
      { id: "122", name: "2026-2027-1", isCurrent: true },
      { id: "123", name: "2026-2027-2", isCurrent: false },
    ]);
    let requestSignal: AbortSignal | undefined;
    let resolveTimeslots: ((value: TimeSlot[]) => void) | undefined;
    apiMocks.timeslots.mockImplementation((_offeringId: string, signal: AbortSignal) => {
      requestSignal = signal;
      return new Promise<TimeSlot[]>((resolve) => {
        resolveTimeslots = resolve;
      });
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" }));
    await waitFor(() => expect(requestSignal).toBeDefined());
    await user.click(screen.getByRole("combobox", { name: "学期" }));
    await user.click(screen.getByRole("option", { name: "2026-2027-2" }));

    await waitFor(() => expect(requestSignal?.aborted).toBe(true));
    resolveTimeslots?.([slot("9001")]);
    await waitFor(() => expect(useScheduleStore.getState().schedules).toEqual({}));
  });

  it("rejects an inverted time range before requesting filtered offerings", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" });
    apiMocks.offerings.mockClear();

    await user.click(screen.getByRole("combobox", { name: "星期" }));
    await user.click(screen.getByRole("option", { name: "周一" }));
    await user.click(screen.getByRole("combobox", { name: "起始节" }));
    await user.click(screen.getByRole("option", { name: "5" }));
    await user.click(screen.getByRole("combobox", { name: "结束节" }));
    await user.click(screen.getByRole("option", { name: "2" }));

    expect(await screen.findAllByText("起始节不能晚于结束节。")).toHaveLength(2);
    await waitFor(() => expect(apiMocks.offerings).not.toHaveBeenCalled());
  });

  it("does not silently ignore a week without a complete time range", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" });
    apiMocks.offerings.mockClear();

    await user.click(screen.getByRole("combobox", { name: "周次" }));
    await user.click(screen.getByRole("option", { name: "第 3 周" }));

    expect(await screen.findByText("请完整选择星期、起始节和结束节。")).toBeVisible();
    await waitFor(() => expect(apiMocks.offerings).not.toHaveBeenCalled());
  });
});
