import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { LoginPage } from "./login-page";

const authMocks = vi.hoisted(() => ({
  requestCode: vi.fn(),
  verifyEmail: vi.fn(),
  loginWithPassword: vi.fn(),
  acceptAuthTokens: vi.fn(),
}));
const apiMocks = vi.hoisted(() => ({
  passwordForgot: vi.fn(),
  passwordReset: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({
    ...authMocks,
    isAuthenticated: false,
  }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: apiMocks,
}));

vi.mock("@/components/common/yourtj-captcha", () => ({
  YourTJCaptcha: ({
    open,
    onVerified,
  }: {
    open: boolean;
    onVerified: (token: string) => void;
  }) => open ? <button type="button" onClick={() => onVerified("captcha-token")}>通过人机验证</button> : null,
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/login?next=/notifications"]}>
        <LoginPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("LoginPage", () => {
  beforeEach(() => {
    authMocks.requestCode.mockReset().mockResolvedValue(undefined);
    authMocks.verifyEmail.mockReset().mockResolvedValue(undefined);
    authMocks.loginWithPassword.mockReset().mockResolvedValue(undefined);
    authMocks.acceptAuthTokens.mockReset().mockResolvedValue(undefined);
    apiMocks.passwordForgot.mockReset().mockResolvedValue(undefined);
    apiMocks.passwordReset.mockReset();
  });

  it("offers password login as the clear default journey", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    await user.type(screen.getByLabelText("同济邮箱"), " Alice@Tongji.edu.cn ");
    await user.type(screen.getByLabelText("密码"), "correct-horse-battery-staple!");
    await user.click(screen.getByRole("button", { name: "使用密码登录" }));

    await waitFor(() => expect(authMocks.loginWithPassword).toHaveBeenCalledWith({
      email: "alice@tongji.edu.cn",
      password: "correct-horse-battery-staple!",
    }));
    await expectNoAccessibilityViolations(view.container);
  });

  it("binds registration codes to registration and requires a non-email-derived handle", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole("tab", { name: "注册账号" }));
    await user.type(screen.getByLabelText("同济邮箱"), "student@tongji.edu.cn");
    await user.click(screen.getByRole("button", { name: "发送注册码" }));
    await user.click(screen.getByRole("button", { name: "通过人机验证" }));

    await waitFor(() => expect(authMocks.requestCode).toHaveBeenCalledWith(
      "student@tongji.edu.cn",
      "captcha-token",
      "registration",
    ));

    await user.type(screen.getByLabelText("注册验证码"), "123456");
    await user.type(screen.getByLabelText("公开 handle"), "campus-reader");
    await user.click(screen.getByRole("button", { name: "验证并注册" }));

    await waitFor(() => expect(authMocks.verifyEmail).toHaveBeenCalledWith({
      email: "student@tongji.edu.cn",
      code: "123456",
      purpose: "registration",
      handle: "campus-reader",
      password: undefined,
    }));
  });

  it("adopts the replacement session returned by a successful password reset", async () => {
    const user = userEvent.setup();
    const tokens = {
      accessToken: "reset-access",
      refreshToken: "reset-refresh",
      account: { id: "1", handle: "owner", hasPassword: true },
    };
    apiMocks.passwordReset.mockResolvedValue(tokens);
    renderPage();

    await user.type(screen.getByLabelText("同济邮箱"), "owner@tongji.edu.cn");
    await user.click(screen.getByRole("button", { name: "忘记密码？" }));
    await user.type(screen.getByLabelText("重置验证码"), "123456");
    await user.type(screen.getByLabelText("新密码"), "correct-horse-battery-staple!");
    await user.type(screen.getByLabelText("确认新密码"), "correct-horse-battery-staple!");
    await user.click(screen.getByRole("button", { name: "重置密码" }));

    await waitFor(() => expect(authMocks.acceptAuthTokens).toHaveBeenCalledWith(tokens));
  });
});
