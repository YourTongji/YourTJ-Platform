import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { AppealsPage } from "./appeals-page";

const apiMocks = vi.hoisted(() => ({
  list: vi.fn(),
  submit: vi.fn(),
  withdraw: vi.fn(),
  notices: vi.fn(),
  markNoticeRead: vi.fn(),
  appealPasswordLogin: vi.fn(),
}));

const authState = vi.hoisted(() => ({
  isAuthenticated: true,
  account: { id: "1" } as { id: string } | null,
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => authState,
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    myAppeals: apiMocks.list,
    submitAppeal: apiMocks.submit,
    withdrawAppeal: apiMocks.withdraw,
    governanceNotices: apiMocks.notices,
    markGovernanceNoticesRead: apiMocks.markNoticeRead,
    appealPasswordLogin: apiMocks.appealPasswordLogin,
  },
}));

vi.mock("@/lib/random", () => ({ randomUuid: () => "stable-appeal-request" }));

const submittedAppeal = {
  id: "51",
  governanceEventId: "44",
  originalAction: "forum.thread.hidden",
  originalReason: "违反社区规则",
  targetKind: "forum_thread" as const,
  targetId: "23",
  dispositionKind: "hide" as const,
  status: "submitted" as const,
  submissionReason: "内容语境被误解",
  submittedAt: 1_720_000_000,
  appealableUntil: 1_722_592_000,
  reviewStartedAt: null,
  decisionReason: null,
  amendment: null,
  decidedAt: null,
  version: 1,
  history: [
    {
      id: "61",
      fromStatus: null,
      toStatus: "submitted" as const,
      reason: "内容语境被误解",
      metadata: null,
      createdAt: 1_720_000_000,
    },
  ],
};

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/appeals?event=44"]}>
        <AppealsPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AppealsPage", () => {
  beforeEach(() => {
    sessionStorage.clear();
    authState.isAuthenticated = true;
    authState.account = { id: "1" };
    apiMocks.list.mockReset().mockResolvedValue({
      items: [submittedAppeal],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.submit.mockReset().mockResolvedValue(submittedAppeal);
    apiMocks.withdraw.mockReset().mockResolvedValue({
      ...submittedAppeal,
      status: "withdrawn",
      version: 2,
    });
    apiMocks.notices.mockReset().mockResolvedValue({
      items: [
        {
          id: "71",
          noticeType: "content_restricted",
          subjectKind: "forum_thread",
          subjectId: "23",
          summary: "你的主题已被隐藏，可申请独立复核。",
          appealId: null,
          targetUrl: "/appeals?event=44",
          read: false,
          readAt: null,
          createdAt: 1_720_000_010,
        },
      ],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.markNoticeRead.mockReset().mockResolvedValue(undefined);
    apiMocks.appealPasswordLogin.mockReset().mockResolvedValue({
      accessToken: "restricted-appeal-token",
      expiresAt: Math.floor(Date.now() / 1_000) + 3600,
    });
  });

  it("shows immutable history and supports idempotent submit and explicit withdrawal", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect(await screen.findByText("违反社区规则")).toBeVisible();
    expect(screen.getByText("你的主题已被隐藏，可申请独立复核。")).toBeVisible();
    expect(screen.getByRole("list", { name: "申诉状态历史" })).toHaveTextContent("内容语境被误解");
    expect(screen.getByLabelText("治理事件编号")).toHaveValue("44");

    await user.click(screen.getByRole("button", { name: "用此事件申诉" }));
    await waitFor(() => expect(apiMocks.markNoticeRead).toHaveBeenCalledWith(["71"], undefined));

    await user.type(screen.getByLabelText("申诉理由"), "请独立复核原处置");
    await user.click(screen.getByRole("button", { name: "提交申诉" }));
    await waitFor(() => expect(apiMocks.submit).toHaveBeenCalledWith(
      { governanceEventId: "44", reason: "请独立复核原处置" },
      "appeal:stable-appeal-request",
      undefined,
    ));

    await user.click(screen.getByRole("button", { name: "撤回申诉" }));
    await user.type(screen.getByLabelText("操作原因"), "暂不继续申诉");
    await user.click(screen.getByRole("button", { name: "确认撤回" }));
    await waitFor(() => expect(apiMocks.withdraw).toHaveBeenCalledWith(
      "51",
      1,
      "暂不继续申诉",
      undefined,
    ));
    await expectNoAccessibilityViolations(view.container);
  });

  it("lets a suspended user use a purpose-bound credential to load notices and appeals", async () => {
    authState.isAuthenticated = false;
    authState.account = null;
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("安全进入申诉中心")).toBeVisible();
    await user.type(screen.getByLabelText("同济邮箱"), "student@tongji.edu.cn");
    await user.type(screen.getByLabelText("密码"), "correct horse battery staple");
    await user.click(screen.getByRole("button", { name: "验证并进入" }));

    await waitFor(() => expect(apiMocks.appealPasswordLogin).toHaveBeenCalledWith({
      email: "student@tongji.edu.cn",
      password: "correct horse battery staple",
    }));
    await waitFor(() => expect(apiMocks.list).toHaveBeenCalledWith(
      null,
      "restricted-appeal-token",
    ));
    await waitFor(() => expect(apiMocks.notices).toHaveBeenCalledWith(
      undefined,
      null,
      "restricted-appeal-token",
    ));
    expect(await screen.findByText("处置与申诉通知")).toBeVisible();
  });

  it("never reuses one restricted account cache after switching credentials", async () => {
    authState.isAuthenticated = false;
    authState.account = null;
    apiMocks.appealPasswordLogin
      .mockReset()
      .mockResolvedValueOnce({
        accessToken: "restricted-account-a",
        expiresAt: Math.floor(Date.now() / 1_000) + 3_600,
      })
      .mockResolvedValueOnce({
        accessToken: "restricted-account-b",
        expiresAt: Math.floor(Date.now() / 1_000) + 3_600,
      });
    apiMocks.list.mockImplementation((_cursor, token) => Promise.resolve({
      items: [{
        ...submittedAppeal,
        id: token === "restricted-account-a" ? "account-a-appeal" : "account-b-appeal",
        originalReason: token === "restricted-account-a"
          ? "账号 A 私密处置"
          : "账号 B 私密处置",
      }],
      hasMore: false,
      nextCursor: null,
    }));
    apiMocks.notices.mockResolvedValue({ items: [], hasMore: false, nextCursor: null });
    const user = userEvent.setup();
    renderPage();

    await user.type(screen.getByLabelText("同济邮箱"), "account-a@tongji.edu.cn");
    await user.type(screen.getByLabelText("密码"), "correct horse battery staple");
    await user.click(screen.getByRole("button", { name: "验证并进入" }));
    expect(await screen.findByText("账号 A 私密处置")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "退出申诉访问" }));
    await waitFor(() => expect(screen.queryByText("账号 A 私密处置")).not.toBeInTheDocument());
    await user.type(screen.getByLabelText("同济邮箱"), "account-b@tongji.edu.cn");
    await user.type(screen.getByLabelText("密码"), "correct horse battery staple");
    await user.click(screen.getByRole("button", { name: "验证并进入" }));

    expect(await screen.findByText("账号 B 私密处置")).toBeVisible();
    expect(screen.queryByText("账号 A 私密处置")).not.toBeInTheDocument();
    expect(apiMocks.list).toHaveBeenCalledWith(null, "restricted-account-b");
  });
});
