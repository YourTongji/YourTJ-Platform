import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { RecentAuthDialog } from "./recent-auth-dialog";

const apiMocks = vi.hoisted(() => ({
  recentAuthStatus: vi.fn(),
  requestRecentAuthCode: vi.fn(),
  verifyRecentAuth: vi.fn(),
}));
const authMocks = vi.hoisted(() => ({ logout: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/context/auth-provider", () => ({ useAuth: () => authMocks }));

function renderDialog(onVerified = vi.fn()) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <RecentAuthDialog open onOpenChange={vi.fn()} onVerified={onVerified} />
      </MemoryRouter>
    </QueryClientProvider>,
  );
  return { ...view, onVerified };
}

describe("RecentAuthDialog", () => {
  beforeEach(() => {
    apiMocks.recentAuthStatus.mockReset().mockResolvedValue({
      sessionBound: true,
      isFresh: false,
      authenticatedAt: null,
      expiresAt: null,
      method: null,
      availableMethods: ["password", "email_code"],
    });
    apiMocks.requestRecentAuthCode.mockReset().mockResolvedValue(undefined);
    apiMocks.verifyRecentAuth.mockReset().mockResolvedValue({
      sessionBound: true,
      isFresh: true,
      authenticatedAt: 1_700_000_000,
      expiresAt: 1_700_000_600,
      method: "password",
      availableMethods: ["password", "email_code"],
    });
    authMocks.logout.mockReset().mockResolvedValue(undefined);
  });

  it("verifies the current password and remains accessible", async () => {
    const user = userEvent.setup();
    const view = renderDialog();

    await user.type(await screen.findByLabelText("当前密码"), "correct horse battery staple");
    await expectNoAccessibilityViolations(view.container);
    await user.click(screen.getByRole("button", { name: "完成验证并继续" }));

    await waitFor(() => expect(apiMocks.verifyRecentAuth).toHaveBeenCalledWith({
      method: "password",
      password: "correct horse battery staple",
    }));
    expect(view.onVerified).toHaveBeenCalledOnce();
  });

  it("requests an account-bound email code without accepting an email", async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.click(await screen.findByRole("tab", { name: "邮箱验证码" }));
    await user.click(screen.getByRole("button", { name: "发送验证码" }));
    await waitFor(() => expect(apiMocks.requestRecentAuthCode).toHaveBeenCalledWith());
    await user.type(screen.getByLabelText("六位验证码"), "123456");
    await user.click(screen.getByRole("button", { name: "完成验证并继续" }));
    await waitFor(() => expect(apiMocks.verifyRecentAuth).toHaveBeenCalledWith({
      method: "email_code",
      code: "123456",
    }));
  });

  it("fails closed for a legacy JWT without a revocable session", async () => {
    apiMocks.recentAuthStatus.mockResolvedValue({
      sessionBound: false,
      isFresh: false,
      authenticatedAt: null,
      expiresAt: null,
      method: null,
      availableMethods: [],
    });
    const view = renderDialog();

    expect(await screen.findByText(/兼容期旧会话/)).toBeVisible();
    expect(screen.getByRole("button", { name: "重新登录" })).toBeVisible();
    await expectNoAccessibilityViolations(view.container);
  });
});
