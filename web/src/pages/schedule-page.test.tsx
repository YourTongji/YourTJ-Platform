import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import { scheduleStorageKey } from "@/lib/schedule-store";

import { SchedulePage } from "./schedule-page";

const apiMocks = vi.hoisted(() => ({
  calendars: vi.fn(),
  latest: vi.fn(),
  grades: vi.fn(),
  majors: vi.fn(),
  natures: vi.fn(),
  byMajor: vi.fn(),
  byNature: vi.fn(),
  search: vi.fn(),
  timeslots: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ account: { id: "account-1" } }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    calendars: apiMocks.calendars,
    selectionLatestUpdate: apiMocks.latest,
    grades: apiMocks.grades,
    majors: apiMocks.majors,
    courseNatures: apiMocks.natures,
    selectionByMajor: apiMocks.byMajor,
    selectionByNature: apiMocks.byNature,
    selectionSearch: apiMocks.search,
    selectionTimeslots: apiMocks.timeslots,
  },
}));

const existingCourse: SelectionCourse = {
  id: "teaching-class-1",
  code: "CS101",
  name: "已有教学班",
  credit: 2,
  natureId: null,
  calendarId: "calendar-1",
  campusId: null,
  teacherName: null,
  teacherNames: [],
};

const candidateCourse: SelectionCourse = {
  ...existingCourse,
  id: "teaching-class-2",
  code: "CS102",
  name: "候选教学班",
};

function slot(courseId: string, weeks: string | null): TimeSlot {
  return {
    courseId,
    teacherName: null,
    weekday: 1,
    startSlot: 1,
    endSlot: 2,
    weeks,
    location: null,
  };
}

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SchedulePage />
    </QueryClientProvider>,
  );
}

describe("SchedulePage possible conflicts", () => {
  beforeEach(() => {
    Element.prototype.scrollIntoView = vi.fn();
    localStorage.clear();
    apiMocks.calendars.mockReset().mockResolvedValue([
      { id: "calendar-1", name: "当前学期", isCurrent: true },
    ]);
    apiMocks.latest.mockReset().mockResolvedValue({ updatedAt: null });
    apiMocks.grades.mockReset().mockResolvedValue([]);
    apiMocks.majors.mockReset().mockResolvedValue([]);
    apiMocks.natures.mockReset().mockResolvedValue([]);
    apiMocks.byMajor.mockReset().mockResolvedValue([]);
    apiMocks.byNature.mockReset().mockResolvedValue([]);
    apiMocks.search.mockReset().mockResolvedValue([candidateCourse]);
    apiMocks.timeslots
      .mockReset()
      .mockResolvedValue([slot(candidateCourse.id, null)]);

    localStorage.setItem(
      scheduleStorageKey({
        environment: "/api/v2",
        principal: "account-1",
        calendarId: "calendar-1",
      }),
      JSON.stringify({
        state: {
          staged: [
            {
              course: existingCourse,
              timeslots: [slot(existingCourse.id, "1-16")],
              color: "#00a08a",
            },
          ],
        },
        version: 2,
      }),
    );
  });

  it("labels an unknown-week overlap as possible and lets the user override it", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("tab", { name: "搜索" }));
    await user.type(screen.getByPlaceholderText("输入课程名、课号或教师"), "数据");
    await user.click(await screen.findByRole("button", { name: "加入" }));

    expect(apiMocks.timeslots).toHaveBeenCalledWith("teaching-class-2");
    expect(await screen.findByRole("dialog")).toHaveTextContent("可能存在周次冲突");
    expect(screen.getByRole("dialog")).toHaveTextContent("周次缺失或无法解析");

    await user.click(screen.getByRole("button", { name: "仍然加入" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.getByText("2 门课程 · 4.0 学分")).toBeInTheDocument();
  });

  it("discards an in-flight add when the selected calendar changes", async () => {
    const user = userEvent.setup();
    let resolveTimeslots: ((slots: TimeSlot[]) => void) | undefined;
    apiMocks.calendars.mockResolvedValue([
      { id: "calendar-1", name: "当前学期", isCurrent: true },
      { id: "calendar-2", name: "下学期", isCurrent: false },
    ]);
    apiMocks.timeslots.mockReturnValue(
      new Promise<TimeSlot[]>((resolve) => {
        resolveTimeslots = resolve;
      }),
    );
    renderPage();

    await user.click(await screen.findByRole("tab", { name: "搜索" }));
    await user.type(screen.getByPlaceholderText("输入课程名、课号或教师"), "数据");
    await user.click(await screen.findByRole("button", { name: "加入" }));
    screen.getAllByRole("combobox")[0]!.focus();
    await user.keyboard("{Enter}{ArrowDown}{Enter}");
    await waitFor(() => {
      expect(screen.getAllByRole("combobox")[0]).toHaveTextContent("下学期");
    });
    resolveTimeslots?.([slot(candidateCourse.id, null)]);

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.getByText("0 门课程 · 0.0 学分")).toBeInTheDocument();
    });
  });
});
