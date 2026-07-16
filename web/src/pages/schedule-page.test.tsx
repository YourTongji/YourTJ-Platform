import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import { API_BASE_URL } from "@/lib/api/client";
import {
  getScheduleStore,
  serializeScheduleExport,
  type ScheduleScope,
} from "@/lib/schedule-store";
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
const toastMocks = vi.hoisted(() => ({
  error: vi.fn(),
  info: vi.fn(),
  success: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("sonner", () => ({ toast: toastMocks }));

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

const scheduleScope: ScheduleScope = {
  environment: API_BASE_URL,
  principal: "anonymous",
  calendarId: "122",
};

function scheduleStore() {
  return getScheduleStore(scheduleScope);
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
    scheduleStore().setState({ staged: [] });
    Object.values(toastMocks).forEach((mock) => mock.mockReset());
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
    expect(scheduleStore().getState().staged[0]?.course.offeringId).toBe("9001");
    await expectNoAccessibilityViolations(view.container);
  });

  it("never offers an override for a confirmed conflict", async () => {
    scheduleStore().getState().addCourse(course("8001", "MATH101"), [slot("8001")]);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" }));
    expect(await screen.findByText("发现课表冲突")).toBeVisible();
    expect(screen.getByText("已确认冲突")).toBeVisible();
    expect(screen.queryByRole("button", { name: "仍然加入并标记" })).not.toBeInTheDocument();
    expect(scheduleStore().getState().staged).toHaveLength(1);
  });

  it("allows an explicit override for a possible conflict", async () => {
    scheduleStore().getState().addCourse(course("8001", "MATH101"), [slot("8001")]);
    apiMocks.timeslots.mockResolvedValue([slot("9001", true)]);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" }));
    expect(await screen.findByText("可能冲突")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "仍然加入并标记" }));

    expect(scheduleStore().getState().staged).toHaveLength(2);
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
      scheduleStore().getState().staged[0]?.course.offeringId,
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
    await waitFor(() => expect(scheduleStore().getState().staged).toEqual([]));
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

  it("imports a validated JSON schedule and reports scope errors", async () => {
    renderPage();
    await screen.findByRole("button", { name: "加入数据结构教学班CS101.01" });
    const input = screen.getByLabelText("导入课表 JSON");
    const validJson = serializeScheduleExport([
      { course: course("7001"), timeslots: [slot("7001")], color: "#007f73" },
    ], scheduleScope);
    const validFile = new File([validJson], "schedule.json", { type: "application/json" });
    Object.defineProperty(validFile, "text", { value: () => Promise.resolve(validJson) });
    Object.defineProperty(validFile, "size", { value: 2 * 1024 * 1024 });

    fireEvent.change(input, { target: { files: [validFile] } });
    await waitFor(() => expect(scheduleStore().getState().staged[0]?.course.offeringId).toBe("7001"));
    expect(toastMocks.success).toHaveBeenCalledWith("已恢复 1 个教学班");

    const invalidJson = serializeScheduleExport([], { ...scheduleScope, calendarId: "999" });
    const invalidFile = new File([invalidJson], "wrong-calendar.json", { type: "application/json" });
    Object.defineProperty(invalidFile, "text", { value: () => Promise.resolve(invalidJson) });
    fireEvent.change(input, { target: { files: [invalidFile] } });

    await waitFor(() => expect(toastMocks.error).toHaveBeenCalledWith(
      "文件格式或环境、学期与当前课表不匹配",
    ));
    expect(scheduleStore().getState().staged[0]?.course.offeringId).toBe("7001");

    const oversizedFile = new File(["{}"], "oversized.json", { type: "application/json" });
    Object.defineProperty(oversizedFile, "size", { value: 2 * 1024 * 1024 + 1 });
    fireEvent.change(input, { target: { files: [oversizedFile] } });
    await waitFor(() => expect(toastMocks.error).toHaveBeenCalledWith(
      "课表 JSON 不能超过 2 MB",
    ));
  });

  it("reports an oversized JSON export without starting a download", async () => {
    const oversizedCourse = course("7001");
    oversizedCourse.facultyName = "x".repeat(2 * 1024 * 1024);
    scheduleStore().setState({
      staged: [{ course: oversizedCourse, timeslots: [slot("7001")], color: "#007f73" }],
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "导出 JSON" }));

    expect(toastMocks.error).toHaveBeenCalledWith("课表 JSON 不能超过 2 MB");
  });

  it("refetches calendar-scoped collections and clears stale search on semester change", async () => {
    apiMocks.calendars.mockResolvedValue([
      { id: "122", name: "2026-2027-1", isCurrent: true },
      { id: "123", name: "2026-2027-2", isCurrent: false },
    ]);
    const user = userEvent.setup();
    renderPage();

    await waitFor(() => expect(apiMocks.natures).toHaveBeenCalledWith("122"));
    await user.click(screen.getByRole("combobox", { name: "学期" }));
    await user.click(screen.getByRole("option", { name: "2026-2027-2" }));

    await waitFor(() => expect(apiMocks.natures).toHaveBeenCalledWith("123"));
    expect(screen.getByRole("textbox", { name: "搜索教学班" })).toHaveValue("");
  });

  it("shows contract-valid twentieth-slot rows", async () => {
    const lateSlot = { ...slot("7001"), startSlot: 20, endSlot: 20 };
    scheduleStore().setState({
      staged: [{ course: course("7001"), timeslots: [lateSlot], color: "#007f73" }],
    });
    renderPage();

    const timetable = await screen.findByRole("region", { name: "每周课表" });
    expect(timetable).toHaveTextContent("20");
  });
});
