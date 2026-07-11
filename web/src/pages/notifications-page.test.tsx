import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { NotificationsPage } from "./notifications-page";

const apiMocks = vi.hoisted(() => ({
  list: vi.fn(),
  markRead: vi.fn(),
  unreadCount: vi.fn(),
  governanceList: vi.fn(),
  governanceMarkRead: vi.fn(),
  governanceUnreadCount: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ isAuthenticated: true }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    notifications: apiMocks.list,
    markNotificationsRead: apiMocks.markRead,
    unreadNotificationCount: apiMocks.unreadCount,
    governanceNotices: apiMocks.governanceList,
    markGovernanceNoticesRead: apiMocks.governanceMarkRead,
    governanceNoticeUnreadCount: apiMocks.governanceUnreadCount,
  },
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <NotificationsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("NotificationsPage", () => {
  beforeEach(() => {
    apiMocks.list.mockReset().mockResolvedValue({
      items: [
        {
          id: "11",
          type: "reply",
          payload: { title: "新的回复", bodyExcerpt: "欢迎参与讨论" },
          targetUrl: "/forum/threads/2",
          read: false,
          readAt: null,
          createdAt: 1_700_000_000,
        },
      ],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.unreadCount.mockReset().mockResolvedValue({ count: 1 });
    apiMocks.markRead.mockReset().mockResolvedValue(undefined);
    apiMocks.governanceList.mockReset().mockResolvedValue({
      items: [
        {
          id: "19",
          noticeType: "disposition_created",
          subjectKind: "forum_thread",
          subjectId: "23",
          summary: "主题已被隐藏：违反社区规则",
          appealId: null,
          targetUrl: "/appeals?event=41",
          read: false,
          readAt: null,
          createdAt: 1_700_000_010,
        },
      ],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.governanceUnreadCount.mockReset().mockResolvedValue({ count: 1 });
    apiMocks.governanceMarkRead.mockReset().mockResolvedValue(undefined);
  });

  it("shows actionable unread notifications and supports selected or all read", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect(await screen.findByText("新的回复")).toBeVisible();
    expect(screen.getByRole("link", { name: "查看通知详情" })).toHaveAttribute(
      "href",
      "/forum/threads/2",
    );
    expect(screen.getByText("主题已被隐藏：违反社区规则")).toBeVisible();
    expect(screen.getByRole("link", { name: "查看治理通知详情" })).toHaveAttribute(
      "href",
      "/appeals?event=41",
    );

    await user.click(screen.getByRole("button", { name: "标记治理通知为已读" }));
    await waitFor(() => expect(apiMocks.governanceMarkRead).toHaveBeenCalledWith(["19"]));

    await user.click(screen.getByRole("button", { name: "标记为已读" }));
    await waitFor(() => expect(apiMocks.markRead).toHaveBeenCalledWith(["11"]));

    await user.click(screen.getByRole("button", { name: "全部已读" }));
    await waitFor(() => expect(apiMocks.markRead).toHaveBeenCalledWith(undefined));
    await waitFor(() => expect(apiMocks.governanceMarkRead).toHaveBeenCalledWith(undefined));
    await expectNoAccessibilityViolations(view.container);
  });
});
