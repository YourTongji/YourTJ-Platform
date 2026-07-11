import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SystemPanel } from "./system-panel";

const apiMocks = vi.hoisted(() => ({
  listOutbox: vi.fn(),
  retryOutbox: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminNotificationOutbox: apiMocks.listOutbox,
    retryAdminNotificationOutbox: apiMocks.retryOutbox,
    triggerSelectionSync: vi.fn(),
    reindexCourses: vi.fn(),
    reindexReviews: vi.fn(),
    reindexForum: vi.fn(),
  },
}));

describe("SystemPanel notification dead letters", () => {
  beforeEach(() => {
    apiMocks.listOutbox.mockReset().mockResolvedValue({
      items: [{
        id: "54",
        topic: "notification",
        recipientAccountId: "9",
        eventType: "reply",
        state: "dead",
        attempts: 8,
        maxAttempts: 8,
        manualRetryCount: 0,
        availableAt: 1_700_000_000,
        lastErrorCode: "database_unavailable",
        completedAt: null,
        deadAt: 1_700_000_100,
        createdAt: 1_700_000_000,
        updatedAt: 1_700_000_100,
      }],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.retryOutbox.mockReset().mockResolvedValue({ id: "54", state: "queued" });
  });

  it("shows safe failure metadata and requires an audited reason before retry", async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={queryClient}>
        <SystemPanel canManageSettings={false} canRunJobs />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("database_unavailable", { exact: false })).toBeVisible();
    expect(screen.queryByText("payload")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "说明原因并重试" }));
    await user.type(screen.getByLabelText("操作原因"), "数据库连接已经恢复");
    await user.click(screen.getByRole("button", { name: "重新排队" }));

    await waitFor(() => {
      expect(apiMocks.retryOutbox).toHaveBeenCalledWith("54", "数据库连接已经恢复");
    });
  });
});
