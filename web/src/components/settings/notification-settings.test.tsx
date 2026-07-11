import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { NotificationSettings } from "./notification-settings";

const apiMocks = vi.hoisted(() => ({
  get: vi.fn(),
  update: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    notificationPrefs: apiMocks.get,
    updateNotificationPrefs: apiMocks.update,
  },
}));

describe("NotificationSettings", () => {
  beforeEach(() => {
    const prefs = {
      inApp: {
        replies: true,
        mentions: true,
        quotes: true,
        votes: true,
        badges: true,
        subscriptions: true,
        directMessages: true,
      },
      email: { weeklyDigest: false },
    };
    apiMocks.get.mockReset().mockResolvedValue({ prefs });
    apiMocks.update.mockReset().mockImplementation(async (updated) => ({ prefs: updated }));
  });

  it("saves a typed event-by-channel preference matrix", async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const view = render(
      <QueryClientProvider client={queryClient}>
        <NotificationSettings />
      </QueryClientProvider>,
    );

    const replies = await screen.findByRole("switch", { name: "站内回复通知" });
    expect(replies).toBeChecked();
    await user.click(replies);
    await user.click(screen.getByRole("button", { name: "保存通知偏好" }));

    await waitFor(() => expect(apiMocks.update).toHaveBeenCalledWith(expect.objectContaining({
      inApp: expect.objectContaining({ replies: false, directMessages: true }),
      email: { weeklyDigest: false },
    })));
    await expectNoAccessibilityViolations(view.container);
  });
});
