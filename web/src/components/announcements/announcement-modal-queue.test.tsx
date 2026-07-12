import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { AnnouncementModalQueue } from "./announcement-modal-queue";

const apiMocks = vi.hoisted(() => ({
  active: vi.fn(),
  receipt: vi.fn(),
  unread: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ isAuthenticated: true }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    announcements: apiMocks.active,
    recordAnnouncementReceipt: apiMocks.receipt,
    unreadAnnouncements: apiMocks.unread,
  },
}));

function renderQueue() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <AnnouncementModalQueue />
    </QueryClientProvider>,
  );
}

describe("AnnouncementModalQueue", () => {
  beforeEach(() => {
    apiMocks.active.mockReset().mockResolvedValue([]);
    apiMocks.receipt.mockReset().mockResolvedValue({
      revision: 3,
      firstSeenAt: 1_700_000_000,
      dismissedAt: null,
      acknowledgedAt: 1_700_000_001,
    });
    apiMocks.unread.mockReset().mockResolvedValue([
      {
        id: "21",
        title: "重要社区规则更新",
        body: "请确认你已经阅读本次规则变更。",
        status: "published",
        effectiveState: "active",
        presentation: "banner",
        severity: "critical",
        priority: 100,
        audience: "authenticated",
        requiresAck: true,
        version: 4,
        revision: 3,
        startsAt: null,
        endsAt: null,
        publishedAt: 1_700_000_000,
        archivedAt: null,
        createdAt: 1_700_000_000,
        updatedAt: 1_700_000_000,
        receipt: null,
        receiptSummary: null,
      },
    ]);
  });

  it("records seen after rendering and requires an explicit acknowledgement action", async () => {
    const user = userEvent.setup();
    const view = renderQueue();

    expect(await screen.findByRole("dialog", { name: "重要社区规则更新" })).toBeVisible();
    await waitFor(() => expect(apiMocks.receipt).toHaveBeenCalledWith("21", {
      revision: 3,
      action: "seen",
    }));
    await expectNoAccessibilityViolations(view.baseElement);

    await user.click(screen.getByRole("button", { name: "我已知晓" }));
    await waitFor(() => expect(apiMocks.receipt).toHaveBeenCalledWith("21", {
      revision: 3,
      action: "acknowledge",
    }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });
});
