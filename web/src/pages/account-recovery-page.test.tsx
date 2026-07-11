import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { AccountRecoveryPage } from "./account-recovery-page";

const apiMocks = vi.hoisted(() => ({
  recoveryPassword: vi.fn(),
  requestEmailCode: vi.fn(),
  recoveryEmailVerify: vi.fn(),
  inspectRecovery: vi.fn(),
  reactivateAccount: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ isAuthenticated: false }),
}));
vi.mock("@/components/common/yourtj-captcha", () => ({
  YourTJCaptcha: ({ open, onVerified }: { open: boolean; onVerified: (token: string) => void }) =>
    open ? <button type="button" onClick={() => onVerified("captcha-token")}>通过人机验证</button> : null,
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/recover-account"]}>
        <AccountRecoveryPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AccountRecoveryPage", () => {
  beforeEach(() => {
    sessionStorage.clear();
    const lifecycle = {
      state: "deletion_requested",
      deactivatedAt: null,
      deletionRequestedAt: 1_700_000_000,
      recoverUntil: 4_100_000_000,
      deletedAt: null,
      purgedAt: null,
      lifecycleVersion: 2,
    };
    apiMocks.recoveryPassword.mockReset().mockResolvedValue({
      recoveryToken: "r".repeat(43),
      expiresAt: 4_100_000_000,
      lifecycle,
    });
    apiMocks.recoveryEmailVerify.mockReset();
    apiMocks.inspectRecovery.mockReset().mockResolvedValue(lifecycle);
    apiMocks.reactivateAccount.mockReset().mockResolvedValue({ ...lifecycle, state: "active" });
    apiMocks.requestEmailCode.mockReset().mockResolvedValue(undefined);
  });

  it("uses a recovery-only credential and requires a separate confirmation before reactivation", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    await user.type(screen.getByLabelText("同济邮箱"), "owner@tongji.edu.cn");
    await user.type(screen.getByLabelText("当前密码"), "correct-password");
    await user.click(screen.getByRole("button", { name: "验证恢复资格" }));

    expect(await screen.findByRole("heading", { name: "确认恢复账号" })).toBeVisible();
    expect(apiMocks.recoveryPassword).toHaveBeenCalledWith({
      email: "owner@tongji.edu.cn",
      password: "correct-password",
    });
    await user.click(screen.getByRole("button", { name: "确认恢复账号" }));
    await waitFor(() => expect(apiMocks.reactivateAccount).toHaveBeenCalledWith("r".repeat(43)));
    await expectNoAccessibilityViolations(view.container);
  });
});
