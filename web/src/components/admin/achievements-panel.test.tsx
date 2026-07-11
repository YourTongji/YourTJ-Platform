import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { AchievementsPanel } from "./achievements-panel";

const apiMocks = vi.hoisted(() => ({
  listDefinitions: vi.fn(),
  createDefinition: vi.fn(),
  updateDefinition: vi.fn(),
  listGrants: vi.fn(),
  grant: vi.fn(),
  revoke: vi.fn(),
  listEvents: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminAchievements: apiMocks.listDefinitions,
    createAdminAchievement: apiMocks.createDefinition,
    updateAdminAchievement: apiMocks.updateDefinition,
    adminUserAchievements: apiMocks.listGrants,
    grantAdminUserAchievement: apiMocks.grant,
    revokeAdminUserAchievement: apiMocks.revoke,
    adminUserAchievementEvents: apiMocks.listEvents,
  },
}));

const definition = {
  id: "7",
  slug: "community-helper",
  name: "社区助手",
  description: "持续帮助社区成员",
  icon: "award" as const,
  status: "active" as const,
  mintAmount: 20,
  version: 3,
  createdAt: 1_700_000_000,
  updatedAt: 1_700_000_100,
};

const grant = {
  accountId: "42",
  achievementId: "7",
  slug: "community-helper",
  name: "社区助手",
  icon: "award" as const,
  definitionStatus: "active" as const,
  status: "active" as const,
  awardReason: "确认持续帮助社区成员",
  awardedAt: 1_700_000_100,
  awardedBy: "1",
  revokedAt: null,
  revokedBy: null,
  revokeReason: null,
};

describe("AchievementsPanel", () => {
  beforeEach(() => {
    apiMocks.listDefinitions.mockReset().mockResolvedValue({
      items: [definition],
      hasMore: false,
    });
    apiMocks.createDefinition.mockReset().mockResolvedValue(definition);
    apiMocks.updateDefinition.mockReset().mockResolvedValue({ ...definition, version: 4 });
    apiMocks.listGrants.mockReset().mockResolvedValue({ items: [grant], hasMore: false });
    apiMocks.grant.mockReset().mockResolvedValue(grant);
    apiMocks.revoke.mockReset().mockResolvedValue({ ...grant, status: "revoked" });
    apiMocks.listEvents.mockReset().mockResolvedValue({
      items: [{
        id: "19",
        achievementId: "7",
        slug: "community-helper",
        name: "社区助手",
        action: "awarded",
        source: "manual",
        actorId: "1",
        reason: "确认持续帮助社区成员",
        createdAt: 1_700_000_100,
      }],
      hasMore: false,
    });
  });

  it("creates and versions definitions while granting and revoking without implying a mint", async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const view = render(
      <QueryClientProvider client={queryClient}>
        <AchievementsPanel initialAccountId="42" />
      </QueryClientProvider>,
    );

    await screen.findByText("确认持续帮助社区成员");
    await user.type(screen.getByLabelText("Slug"), "helpful-reviewer");
    await user.type(screen.getByLabelText("展示名称"), "课评贡献者");
    await user.type(screen.getByLabelText("操作原因"), "建立课评贡献里程碑");
    await user.click(screen.getByRole("button", { name: "创建成就" }));
    await waitFor(() => expect(apiMocks.createDefinition).toHaveBeenCalledWith(expect.objectContaining({
      slug: "helpful-reviewer",
      name: "课评贡献者",
      icon: "award",
      mintAmount: 0,
      reason: "建立课评贡献里程碑",
    })));

    await user.click(screen.getByRole("button", { name: "编辑" }));
    await user.clear(screen.getByLabelText("展示名称"));
    await user.type(screen.getByLabelText("展示名称"), "社区贡献者");
    await user.type(screen.getByLabelText("操作原因"), "更新公开展示名称");
    await user.click(screen.getByRole("button", { name: "保存定义" }));
    await waitFor(() => expect(apiMocks.updateDefinition).toHaveBeenCalledWith("7", expect.objectContaining({
      expectedVersion: 3,
      name: "社区贡献者",
      status: "active",
      reason: "更新公开展示名称",
    })));

    await user.type(screen.getByLabelText("授予原因"), "人工确认长期贡献");
    await user.click(screen.getByRole("button", { name: "授予成就" }));
    await waitFor(() => expect(apiMocks.grant).toHaveBeenCalledWith("42", {
      achievementId: "7",
      reason: "人工确认长期贡献",
    }));

    await user.click(screen.getByRole("button", { name: "撤销" }));
    const revokeDialog = screen.getByRole("dialog");
    await user.type(within(revokeDialog).getByLabelText("操作原因"), "记录授予对象选择错误");
    await user.click(within(revokeDialog).getByRole("button", { name: "确认撤销" }));
    await waitFor(() => expect(apiMocks.revoke).toHaveBeenCalledWith(
      "42",
      "7",
      "记录授予对象选择错误",
    ));
    expect(screen.getAllByText(/不会/).length).toBeGreaterThan(0);
    await expectNoAccessibilityViolations(view.container);
  });
});
