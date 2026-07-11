import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api/client";

import { UsersPanel } from "./users-panel";

const apiMocks = vi.hoisted(() => ({
  adminUsers: vi.fn(),
  adminUserSanctions: vi.fn(),
  revokeAdminUserSessions: vi.fn(),
  updateAdminUserRole: vi.fn(),
  sanctionAdminUser: vi.fn(),
  unsanctionAdminUser: vi.fn(),
  recentAuthStatus: vi.fn(),
  requestRecentAuthCode: vi.fn(),
  verifyRecentAuth: vi.fn(),
  inviteAdminUser: vi.fn(),
}));
const authMocks = vi.hoisted(() => ({
  account: { id: "1", handle: "admin", role: "admin", capabilities: ["users.search", "users.suspend"] },
  logout: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/context/auth-provider", () => ({ useAuth: () => authMocks }));

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <UsersPanel capabilities={new Set(["users.search", "users.suspend"])} />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("UsersPanel recent-auth recovery", () => {
  beforeEach(() => {
    apiMocks.adminUsers.mockReset().mockResolvedValue({
      items: [{
        id: "2",
        handle: "target",
        avatarUrl: null,
        role: "user",
        status: "active",
        trustLevel: 0,
        lastActiveAt: null,
        createdAt: 1_700_000_000,
      }],
      nextCursor: null,
      hasMore: false,
    });
    apiMocks.adminUserSanctions.mockReset().mockResolvedValue([]);
    apiMocks.revokeAdminUserSessions
      .mockReset()
      .mockRejectedValueOnce(new ApiError(428, "recent authentication required", "RECENT_AUTH_REQUIRED"))
      .mockResolvedValueOnce(undefined);
    apiMocks.updateAdminUserRole.mockReset().mockResolvedValue(undefined);
    apiMocks.sanctionAdminUser.mockReset().mockResolvedValue(undefined);
    apiMocks.unsanctionAdminUser.mockReset().mockResolvedValue(undefined);
    apiMocks.recentAuthStatus.mockReset().mockResolvedValue({
      sessionBound: true,
      isFresh: false,
      authenticatedAt: null,
      expiresAt: null,
      method: null,
      availableMethods: ["password"],
    });
    apiMocks.verifyRecentAuth.mockReset().mockResolvedValue({
      sessionBound: true,
      isFresh: true,
      authenticatedAt: 1_700_000_000,
      expiresAt: 1_700_000_600,
      method: "password",
      availableMethods: ["password"],
    });
    apiMocks.requestRecentAuthCode.mockReset().mockResolvedValue(undefined);
    apiMocks.inviteAdminUser.mockReset().mockResolvedValue(undefined);
    authMocks.logout.mockReset().mockResolvedValue(undefined);
  });

  it("retries a server-rejected high-risk action only after step-up succeeds", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(await screen.findByRole("button", { name: "撤销会话" }));
    const reasonDialog = screen.getByRole("dialog");
    await user.type(within(reasonDialog).getByLabelText("操作原因"), "suspected credential compromise");
    await user.click(within(reasonDialog).getByRole("button", { name: "确认执行" }));

    expect(await screen.findByRole("heading", { name: "确认是你本人" })).toBeVisible();
    await user.type(screen.getByLabelText("当前密码"), "correct horse battery staple");
    await user.click(screen.getByRole("button", { name: "完成验证并继续" }));

    await waitFor(() => expect(apiMocks.revokeAdminUserSessions).toHaveBeenCalledTimes(2));
    expect(apiMocks.revokeAdminUserSessions).toHaveBeenLastCalledWith(
      "2",
      "suspected credential compromise",
    );
  });
});
