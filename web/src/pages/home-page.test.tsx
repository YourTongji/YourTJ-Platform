import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { HomePage } from "./home-page";

const apiMocks = vi.hoisted(() => ({
  announcements: vi.fn(),
  checkIn: vi.fn(),
  myActivity: vi.fn(),
  myCheckInStatus: vi.fn(),
  myTrustProgress: vi.fn(),
  threads: vi.fn(),
}));
const authState = vi.hoisted(() => ({
  account: null as null | { id: string; handle: string; trustLevel: number },
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => authState,
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    announcements: apiMocks.announcements,
    checkIn: apiMocks.checkIn,
    myCheckInStatus: apiMocks.myCheckInStatus,
    myTrustProgress: apiMocks.myTrustProgress,
    threads: apiMocks.threads,
    myActivity: apiMocks.myActivity,
  },
}));

const firstThread = {
  id: "21",
  boardId: "1",
  authorHandle: "alice",
  authorAvatar: null,
  title: "首页第一页",
  bodyExcerpt: "第一页摘要",
  contentVersion: 1,
  replyCount: 2,
  voteCount: 5,
  hotScore: 3,
  status: "visible" as const,
  createdAt: 1_700_000_000,
  lastActivityAt: 1_700_000_100,
  tags: [],
  canEdit: false,
  canDelete: false,
  canModerate: false,
};

function renderPage(queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })) {
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("HomePage", () => {
  beforeEach(() => {
    authState.account = null;
    apiMocks.announcements.mockReset().mockResolvedValue([]);
    apiMocks.checkIn.mockReset();
    apiMocks.myActivity.mockReset().mockResolvedValue({
      timezone: "Asia/Shanghai",
      from: "2026-02-23",
      to: "2026-07-12",
      policyVersion: 1,
      trustPolicyVersion: 1,
      weights: { thread: 10, comment: 3, like: 1, checkIn: 1 },
      likeDailyCap: 20,
      days: [],
    });
    apiMocks.myCheckInStatus.mockReset().mockResolvedValue({
      timezone: "Asia/Shanghai",
      date: "2026-07-12",
      checkedIn: false,
      newlyCheckedIn: false,
      checkedInAt: null,
      currentStreak: 2,
      totalDays: 8,
      nextResetAt: 1_784_044_800,
    });
    apiMocks.myTrustProgress.mockReset().mockResolvedValue({
      trustLevel: 2,
      teaName: "白茶",
      qualifyingScore: 42,
      nextLevel: 3,
      nextThreshold: 120,
      remainingScore: 78,
      progressPercent: 35,
      policyVersion: 1,
      isMaxLevel: false,
      overrideActive: false,
      promotionBlockedUntil: null,
      promotionRequiresNewActivity: false,
    });
    apiMocks.threads.mockReset().mockImplementation(async ({ cursor }) => cursor
      ? {
          items: [{ ...firstThread, id: "20", title: "首页第二页" }],
          nextCursor: null,
          hasMore: false,
        }
      : {
          items: [firstThread],
          nextCursor: "cursor-21",
          hasMore: true,
        });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("continues the active home feed with the server cursor", async () => {
    const user = userEvent.setup();
    renderPage();

    expect((await screen.findAllByText("首页第一页"))[0]).toBeVisible();
    await user.click(screen.getByRole("button", { name: "加载更多动态" }));

    expect((await screen.findAllByText("首页第二页"))[0]).toBeVisible();
    await waitFor(() => expect(apiMocks.threads).toHaveBeenLastCalledWith({
      feed: "hot",
      cursor: "cursor-21",
    }));
  });

  it("shows activity-based trust progress in the narrow-screen home card", async () => {
    authState.account = { id: "7", handle: "alice", trustLevel: 2 };
    renderPage();

    const mobileCard = await screen.findByLabelText("移动端每日签到与成长");
    await waitFor(() => {
      expect(within(mobileCard).getByText("距离 Lv.3 还需 78 分")).toBeVisible();
    });
    expect(within(mobileCard).getByText("42 分")).toBeVisible();
    expect(within(mobileCard).getByRole("link", { name: "查看个人成长" })).toHaveAttribute(
      "href",
      "/profile/alice",
    );
  });

  it("reuses fresh check-in state when the home route remounts", async () => {
    authState.account = { id: "7", handle: "alice", trustLevel: 2 };
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const firstView = renderPage(queryClient);
    await waitFor(() => expect(apiMocks.myCheckInStatus).toHaveBeenCalledTimes(1));

    firstView.unmount();
    renderPage(queryClient);
    await screen.findAllByRole("button", { name: /每日签到/ });

    expect(apiMocks.myCheckInStatus).toHaveBeenCalledTimes(1);
  });

  it("refreshes check-in, activity range, and trust progress after Shanghai midnight", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-12T15:59:58Z"));
    authState.account = { id: "7", handle: "alice", trustLevel: 2 };
    apiMocks.myCheckInStatus
      .mockReset()
      .mockResolvedValueOnce({
        timezone: "Asia/Shanghai",
        date: "2026-07-12",
        checkedIn: true,
        newlyCheckedIn: false,
        checkedInAt: 1_783_870_000,
        currentStreak: 2,
        totalDays: 8,
        nextResetAt: 1_783_872_000,
      })
      .mockResolvedValue({
        timezone: "Asia/Shanghai",
        date: "2026-07-13",
        checkedIn: false,
        newlyCheckedIn: false,
        checkedInAt: null,
        currentStreak: 2,
        totalDays: 8,
        nextResetAt: 1_783_958_400,
    });

    renderPage();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(apiMocks.myCheckInStatus).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(3_100);
    });

    expect(apiMocks.myCheckInStatus).toHaveBeenCalledTimes(2);
    expect(apiMocks.myActivity).toHaveBeenLastCalledWith(expect.any(String), "2026-07-13");
    expect(apiMocks.myTrustProgress).toHaveBeenCalledTimes(2);
  });
});
