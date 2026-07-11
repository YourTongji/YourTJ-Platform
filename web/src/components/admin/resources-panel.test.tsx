import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ResourcesPanel } from "./resources-panel";

const apiMocks = vi.hoisted(() => ({
  listUploads: vi.fn(),
  createPreviewGrant: vi.fn(),
  preview: vi.fn(),
  moderate: vi.fn(),
}));
const originalCreateObjectURL = URL.createObjectURL;
const originalRevokeObjectURL = URL.revokeObjectURL;
let previewEvidenceRecorded = false;

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminMediaUploads: apiMocks.listUploads,
    createAdminMediaPreviewGrant: apiMocks.createPreviewGrant,
    adminMediaPreview: apiMocks.preview,
    moderateAdminMediaUpload: apiMocks.moderate,
  },
}));

const upload = {
  id: "42",
  accountId: "7",
  kind: "image" as const,
  bytes: 128,
  mime: "image/png",
  status: "pending" as const,
  usage: "forum_thread" as const,
  imageWidth: null,
  imageHeight: null,
  approvalRequirement: "image_preview" as const,
  deletionState: null,
  createdAt: 1_700_000_000,
};

describe("ResourcesPanel media moderation", () => {
  beforeEach(() => {
    previewEvidenceRecorded = false;
    apiMocks.listUploads.mockReset().mockImplementation(async () => ({
      items: [{
        ...upload,
        approvalRequirement: previewEvidenceRecorded ? "satisfied" as const : "image_preview" as const,
      }],
      nextCursor: null,
      hasMore: false,
    }));
    apiMocks.createPreviewGrant.mockReset().mockResolvedValue({
      token: "a".repeat(43),
      expiresAt: 1_700_000_060,
    });
    apiMocks.preview.mockReset().mockImplementation(async () => {
      previewEvidenceRecorded = true;
      return new Blob(["png"], { type: "image/png" });
    });
    apiMocks.moderate.mockReset().mockResolvedValue({ ok: true });
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: vi.fn(() => "blob:moderation-preview"),
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });
  });

  afterEach(() => {
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: originalCreateObjectURL,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: originalRevokeObjectURL,
    });
  });

  it("requires an audited reason and renders only the proxied browser blob", async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const view = render(
      <QueryClientProvider client={queryClient}>
        <ResourcesPanel capabilities={new Set(["moderation.content"])} />
      </QueryClientProvider>,
    );

    await user.click(await screen.findByRole("button", { name: "安全预览" }));
    await user.type(screen.getByLabelText("操作原因"), "核对待审图片内容");
    await user.click(screen.getByRole("button", { name: "生成并读取预览" }));

    await waitFor(() => {
      expect(apiMocks.createPreviewGrant).toHaveBeenCalledWith("42", "核对待审图片内容");
      expect(apiMocks.preview).toHaveBeenCalledWith("42", "a".repeat(43));
    });
    expect(await screen.findByRole("img", { name: "待审上传 42 的一次性预览" })).toHaveAttribute(
      "src",
      "blob:moderation-preview",
    );
    expect(view.container).not.toHaveTextContent("ossKey");
    expect(view.container).not.toHaveTextContent("sha256");
    expect(view.container).not.toHaveTextContent("aliyuncs.com");
    await expectNoAccessibilityViolations(view.container);
  });

  it("keeps approval disabled until the same reviewer records trusted preview evidence", async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    render(
      <QueryClientProvider client={queryClient}>
        <ResourcesPanel capabilities={new Set(["moderation.content"])} />
      </QueryClientProvider>,
    );

    const approve = await screen.findByRole("button", { name: "批准" });
    expect(approve).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "安全预览" }));
    await user.type(screen.getByLabelText("操作原因"), "检查图片完整内容");
    await user.click(screen.getByRole("button", { name: "生成并读取预览" }));
    await waitFor(() => expect(approve).toBeEnabled());

    await user.click(approve);
    await user.type(screen.getByLabelText("操作原因"), "图片符合社区内容规范");
    await user.click(screen.getByRole("button", { name: "确认批准" }));
    await waitFor(() => {
      expect(apiMocks.moderate).toHaveBeenCalledWith("42", "approve", "图片符合社区内容规范");
    });

    await user.click(screen.getByRole("button", { name: "已发布" }));
    await waitFor(() => expect(apiMocks.listUploads).toHaveBeenCalledWith(null, "clean"));
  });
});
