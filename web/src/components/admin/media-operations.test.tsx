import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api/client";
import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { MediaOperations } from "./media-operations";

const apiMocks = vi.hoisted(() => ({
  listHolds: vi.fn(),
  listJobs: vi.fn(),
  placeHold: vi.fn(),
  releaseHold: vi.fn(),
  retryJob: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminMediaRetentionHolds: apiMocks.listHolds,
    adminMediaDeletionJobs: apiMocks.listJobs,
    placeAdminMediaRetentionHold: apiMocks.placeHold,
    releaseAdminMediaRetentionHold: apiMocks.releaseHold,
    retryAdminMediaDeletionJob: apiMocks.retryJob,
  },
}));

vi.mock("@/components/auth/recent-auth-dialog", () => ({
  RecentAuthDialog: ({ open, onVerified }: { open: boolean; onVerified: () => void }) => open ? (
    <div role="dialog" aria-label="重新验证身份">
      <button type="button" onClick={onVerified}>验证完成</button>
    </div>
  ) : null,
}));

const hold = {
  id: "501",
  uploadId: "42",
  accountId: "7",
  uploadStatus: "quarantined" as const,
  holdKind: "security" as const,
  reason: "保全安全事件相关媒体证据",
  placedBy: "9",
  expiresAt: Math.floor(Date.now() / 1000) + 24 * 60 * 60,
  createdAt: Math.floor(Date.now() / 1000) - 60 * 60,
  isExpired: false,
};

const deadLetter = {
  id: "88",
  uploadId: "42",
  accountId: "7",
  uploadStatus: "quarantined" as const,
  requestSource: "account_purge" as const,
  reason: "account lifecycle media purge",
  status: "dead_letter" as const,
  attemptCount: 8,
  lastErrorCode: "provider_delete_failed",
  availableAt: Math.floor(Date.now() / 1000),
  createdAt: Math.floor(Date.now() / 1000) - 7_200,
  updatedAt: Math.floor(Date.now() / 1000) - 3_600,
};

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <MediaOperations />
    </QueryClientProvider>,
  );
  return { ...view, queryClient };
}

describe("MediaOperations", () => {
  beforeEach(() => {
    apiMocks.listHolds.mockReset().mockResolvedValue({ items: [hold], nextCursor: null, hasMore: false });
    apiMocks.listJobs.mockReset().mockResolvedValue({ items: [deadLetter], nextCursor: null, hasMore: false });
    apiMocks.placeHold.mockReset().mockResolvedValue(undefined);
    apiMocks.releaseHold.mockReset().mockResolvedValue(undefined);
    apiMocks.retryJob.mockReset().mockResolvedValue(undefined);
  });

  it("requires recent authentication before rendering purpose-bearing inventory", async () => {
    const user = userEvent.setup();
    apiMocks.listHolds
      .mockRejectedValueOnce(new ApiError(428, "recent authentication required", "RECENT_AUTH_REQUIRED"))
      .mockResolvedValueOnce({ items: [hold], nextCursor: null, hasMore: false });

    const view = renderPanel();

    expect(await screen.findByRole("dialog", { name: "重新验证身份" })).toBeInTheDocument();
    expect(screen.queryByText(hold.reason)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "验证完成" }));

    expect(await screen.findByText(hold.reason)).toBeInTheDocument();
    expect(apiMocks.listHolds).toHaveBeenCalledTimes(2);
    await expectNoAccessibilityViolations(view.container);
  });

  it("hides cached purpose details when a later inventory read requires authentication", async () => {
    const view = renderPanel();
    expect(await screen.findByText(hold.reason)).toBeInTheDocument();
    apiMocks.listHolds.mockRejectedValueOnce(
      new ApiError(428, "recent authentication required", "RECENT_AUTH_REQUIRED"),
    );

    await view.queryClient.invalidateQueries({
      queryKey: ["admin", "media", "retention-holds"],
    });

    expect(await screen.findByRole("dialog", { name: "重新验证身份" })).toBeInTheDocument();
    expect(screen.queryByText(hold.reason)).not.toBeInTheDocument();
  });

  it("requires recent authentication when only the deletion inventory rejects", async () => {
    const user = userEvent.setup();
    apiMocks.listJobs
      .mockRejectedValueOnce(new ApiError(428, "recent authentication required", "RECENT_AUTH_REQUIRED"))
      .mockResolvedValueOnce({ items: [deadLetter], nextCursor: null, hasMore: false });
    const view = renderPanel();

    expect(await screen.findByRole("dialog", { name: "重新验证身份" })).toBeInTheDocument();
    expect(screen.queryByText(deadLetter.reason)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "验证完成" }));

    expect(await screen.findByText(deadLetter.reason)).toBeInTheDocument();
    expect(apiMocks.listJobs).toHaveBeenCalledTimes(2);
    await expectNoAccessibilityViolations(view.container);
  });

  it("places a compare-and-swap hold for an exact reviewed upload id", async () => {
    const user = userEvent.setup();
    renderPanel();
    await screen.findByText(hold.reason);

    await user.type(screen.getByLabelText("按已复核上传 ID 设置保留"), "90001");
    await user.click(screen.getAllByRole("button", { name: "设置保留" })[0]);
    expect(screen.getByRole("dialog", { name: "为上传 #90001 设置保留" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("操作原因"), "安全事件需要保留该对象以完成调查");
    await user.click(screen.getByRole("button", { name: "确认设置保留" }));

    await waitFor(() => expect(apiMocks.placeHold).toHaveBeenCalledTimes(1));
    expect(apiMocks.placeHold).toHaveBeenCalledWith("90001", expect.objectContaining({
      holdKind: "moderation",
      expectedHoldId: null,
      reason: "安全事件需要保留该对象以完成调查",
    }));
  });

  it("renews and releases exactly the reviewed hold id", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(await screen.findByRole("button", { name: "续期" }));
    expect(screen.getByLabelText("保留目的")).toHaveTextContent("安全事件");
    await user.type(screen.getByLabelText("操作原因"), "安全事件仍在调查，延长证据保留");
    await user.click(screen.getByRole("button", { name: "确认续期" }));

    await waitFor(() => expect(apiMocks.placeHold).toHaveBeenCalledTimes(1));
    expect(apiMocks.placeHold).toHaveBeenCalledWith("42", expect.objectContaining({
      holdKind: "security",
      expectedHoldId: "501",
      reason: "安全事件仍在调查，延长证据保留",
    }));

    await user.click(await screen.findByRole("button", { name: "解除" }));
    await user.type(screen.getByLabelText("操作原因"), "调查已完成，可以继续清理媒体");
    await user.click(screen.getByRole("button", { name: "确认解除" }));

    await waitFor(() => {
      expect(apiMocks.releaseHold).toHaveBeenCalledWith(
        "42",
        "501",
        "调查已完成，可以继续清理媒体",
      );
    });
  });

  it("retries a dead letter with the same audited reason after recent authentication", async () => {
    const user = userEvent.setup();
    apiMocks.retryJob
      .mockRejectedValueOnce(new ApiError(428, "recent authentication required", "RECENT_AUTH_REQUIRED"))
      .mockResolvedValueOnce(undefined);
    renderPanel();

    await user.click(await screen.findByRole("button", { name: "重新排队" }));
    await user.type(screen.getByLabelText("操作原因"), "存储服务恢复后重试删除任务");
    await user.click(screen.getByRole("button", { name: "确认重新排队" }));
    expect(await screen.findByRole("dialog", { name: "重新验证身份" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "验证完成" }));

    await waitFor(() => expect(apiMocks.retryJob).toHaveBeenCalledTimes(2));
    expect(apiMocks.retryJob).toHaveBeenLastCalledWith("88", "存储服务恢复后重试删除任务");
  });

  it("labels expired upload-intent cleanup without exposing a provider key", async () => {
    apiMocks.listJobs.mockResolvedValue({
      items: [{ ...deadLetter, requestSource: "intent_cleanup" as const }],
      nextCursor: null,
      hasMore: false,
    });
    const view = renderPanel();

    expect(await screen.findByText("过期上传凭证清理")).toBeInTheDocument();
    expect(view.container).not.toHaveTextContent("oss_key");
    expect(view.container).not.toHaveTextContent("aliyuncs.com");
  });
});
