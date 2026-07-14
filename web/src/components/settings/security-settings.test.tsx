import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { SecuritySettings } from "./security-settings";

const apiMocks = vi.hoisted(() => ({
  sessions: vi.fn(),
  revokeSession: vi.fn(),
  revokeOtherSessions: vi.fn(),
  passwordChange: vi.fn(),
  passwordSet: vi.fn(),
}));
const authMocks = vi.hoisted(() => ({
  account: { id: "1", hasPassword: true } as { id: string; hasPassword: boolean },
  acceptAuthTokens: vi.fn(),
  logoutAll: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: apiMocks,
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => authMocks,
}));

function renderSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <SecuritySettings />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("SecuritySettings", () => {
  beforeEach(() => {
    apiMocks.sessions.mockReset().mockResolvedValue({
      items: [
        {
          id: "current",
          isCurrent: true,
          deviceLabel: "Chrome on macOS",
          createdAt: 1_700_000_000,
          lastUsedAt: 1_700_000_100,
          expiresAt: 1_700_100_000,
        },
        {
          id: "other",
          isCurrent: false,
          deviceLabel: "Mobile Safari",
          createdAt: 1_700_000_000,
          lastUsedAt: 1_700_000_050,
          expiresAt: 1_700_100_000,
        },
      ],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.revokeSession.mockReset().mockResolvedValue(undefined);
    apiMocks.revokeOtherSessions.mockReset().mockResolvedValue(undefined);
    apiMocks.passwordChange.mockReset().mockResolvedValue(undefined);
    apiMocks.passwordSet.mockReset().mockResolvedValue(undefined);
    authMocks.account = { id: "1", hasPassword: true };
    authMocks.acceptAuthTokens.mockReset().mockResolvedValue(undefined);
    authMocks.logoutAll.mockReset().mockResolvedValue(undefined);
  });

  it("distinguishes the current device and revokes another session", async () => {
    const user = userEvent.setup();
    const view = renderSettings();

    expect(await screen.findByText("Chrome on macOS")).toBeVisible();
    expect(screen.getByText("当前会话")).toBeVisible();
    expect(screen.getByText("Mobile Safari")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "撤销" }));
    await waitFor(() => expect(apiMocks.revokeSession).toHaveBeenCalledWith("other"));
    await expectNoAccessibilityViolations(view.container);
  });

  it("lets a code-only account establish its first password without asking for an old one", async () => {
    const user = userEvent.setup();
    authMocks.account = { id: "1", hasPassword: false };
    const tokens = {
      accessToken: "new-access",
      refreshToken: "new-refresh",
      account: { id: "1", hasPassword: true },
    };
    apiMocks.passwordSet.mockResolvedValue(tokens);
    const view = renderSettings();

    expect(screen.queryByLabelText("当前密码")).not.toBeInTheDocument();
    await user.type(screen.getByLabelText("新密码"), "correct-horse-battery-staple!");
    await user.type(screen.getByLabelText("确认新密码"), "correct-horse-battery-staple!");
    await user.click(screen.getByRole("button", { name: "设置密码" }));

    await waitFor(() => expect(apiMocks.passwordSet).toHaveBeenCalledWith({
      newPassword: "correct-horse-battery-staple!",
    }));
    expect(authMocks.acceptAuthTokens).toHaveBeenCalledWith(tokens);
    await expectNoAccessibilityViolations(view.container);
  });
});
