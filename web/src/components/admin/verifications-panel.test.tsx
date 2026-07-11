import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { VerificationsPanel } from "./verifications-panel";

const apiMocks = vi.hoisted(() => ({
  listTypes: vi.fn(),
  createType: vi.fn(),
  listGrants: vi.fn(),
  grant: vi.fn(),
  revoke: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminVerificationTypes: apiMocks.listTypes,
    createAdminVerificationType: apiMocks.createType,
    adminUserVerifications: apiMocks.listGrants,
    grantAdminUserVerification: apiMocks.grant,
    revokeAdminUserVerification: apiMocks.revoke,
  },
}));

const verificationType = {
  id: "7",
  slug: "official-organization",
  category: "identity" as const,
  label: "官方学生组织",
  description: "已核实的学生组织账号",
  icon: "building-2" as const,
  badgeVariant: "default" as const,
  allowsPublicDisplay: true,
  createdAt: 1_700_000_000,
};

const activeGrant = {
  id: "17",
  accountId: "42",
  verificationTypeId: "7",
  slug: "official-organization",
  category: "identity" as const,
  label: "官方学生组织",
  icon: "building-2" as const,
  badgeVariant: "default" as const,
  displayOnProfile: true,
  status: "active" as const,
  issuedBy: "1",
  issuedAt: 1_700_000_000,
  expiresAt: null,
  issueReason: "已核实组织所有权",
  hasEvidence: true,
  revokedBy: null,
  revokedAt: null,
  revokeReason: null,
};

describe("VerificationsPanel", () => {
  beforeEach(() => {
    apiMocks.listTypes.mockReset().mockResolvedValue({ items: [verificationType], hasMore: false });
    apiMocks.createType.mockReset().mockResolvedValue(verificationType);
    apiMocks.listGrants.mockReset().mockResolvedValue({ items: [activeGrant], hasMore: false });
    apiMocks.grant.mockReset().mockResolvedValue(activeGrant);
    apiMocks.revoke.mockReset().mockResolvedValue({ ...activeGrant, status: "revoked" });
  });

  it("creates definitions and grants or revokes credentials through reasoned controls", async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const view = render(
      <QueryClientProvider client={queryClient}>
        <VerificationsPanel initialAccountId="42" />
      </QueryClientProvider>,
    );

    await screen.findByText(/已核实组织所有权/);
    await user.type(screen.getByLabelText("Slug"), "verified-contributor");
    await user.type(screen.getByLabelText("公开标签"), "认证贡献者");
    await user.type(screen.getByLabelText("创建原因"), "建立人工认证定义");
    await user.click(screen.getByRole("checkbox", { name: /允许公开展示/ }));
    await user.click(screen.getByRole("button", { name: "创建认证类型" }));
    await waitFor(() => expect(apiMocks.createType).toHaveBeenCalledWith(expect.objectContaining({
      slug: "verified-contributor",
      label: "认证贡献者",
      icon: "badge-check",
      badgeVariant: "default",
      allowsPublicDisplay: true,
      reason: "建立人工认证定义",
    })));

    await user.click(screen.getByRole("checkbox", { name: /显示在公开主页/ }));
    await user.type(screen.getByLabelText("证据引用（可选）"), "case:2026-001");
    await user.type(screen.getByLabelText("授予原因"), "完成组织身份核验");
    await user.click(screen.getByRole("button", { name: "授予认证" }));
    await waitFor(() => expect(apiMocks.grant).toHaveBeenCalledWith("42", expect.objectContaining({
      verificationTypeId: "7",
      displayOnProfile: true,
      evidenceReference: "case:2026-001",
      reason: "完成组织身份核验",
    })));

    await user.click(screen.getByRole("button", { name: "撤销认证" }));
    await user.type(screen.getByLabelText("操作原因"), "认证材料已经失效");
    await user.click(screen.getByRole("button", { name: "确认撤销认证" }));
    await waitFor(() => expect(apiMocks.revoke).toHaveBeenCalledWith("17", "认证材料已经失效"));
    await expectNoAccessibilityViolations(view.container);
  });
});
