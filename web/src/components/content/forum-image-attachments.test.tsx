import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ForumImageAttachments } from "./forum-image-attachments";

const apiMocks = vi.hoisted(() => ({ myMediaUpload: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/components/media/media-upload-button", () => ({
  MediaUploadButton: ({ onUploaded }: { onUploaded: (upload: {
    uploadId: string;
    ossKey: string;
    originalName: string;
  }) => void }) => (
    <button
      type="button"
      onClick={() => onUploaded({
        uploadId: "14",
        ossKey: "private-key-must-not-render",
        originalName: "校园 风景.png",
      })}
    >
      添加图片
    </button>
  ),
}));

function upload(id: string, status: "pending" | "clean" | "blocked") {
  return {
    id,
    kind: "image" as const,
    usage: "forum_thread" as const,
    bytes: 128,
    mime: "image/png",
    status,
    imageWidth: null,
    imageHeight: null,
    createdAt: 1_700_000_000,
  };
}

describe("ForumImageAttachments", () => {
  beforeEach(() => {
    apiMocks.myMediaUpload.mockReset().mockImplementation((id: string) => {
      const status = id === "11" ? "pending" : id === "12" ? "clean" : "blocked";
      return Promise.resolve(upload(id, status));
    });
  });

  it("shows persistent review states, inserts uploads, removes blocked items, and leaks no storage key", async () => {
    const user = userEvent.setup();
    const onUpload = vi.fn();
    const onRemove = vi.fn();
    const onReadyChange = vi.fn();
    const view = render(
      <ForumImageAttachments
        usage="forum_thread"
        assetIds={["11", "12", "13"]}
        maxImages={8}
        onUpload={onUpload}
        onRemove={onRemove}
        onReadyChange={onReadyChange}
      />,
    );

    expect(await screen.findByText("等待安全处理，暂不可发布")).toBeVisible();
    expect(screen.getByText(/GIF 或其他动图请转换为静态图片后重新上传/)).toBeVisible();
    expect(screen.getByText("安全版本已就绪，可发布")).toBeVisible();
    expect(screen.getByText("未通过，请移除")).toBeVisible();
    expect(view.container).not.toHaveTextContent("private-key-must-not-render");
    expect(onReadyChange).toHaveBeenLastCalledWith(false);

    await user.click(screen.getByRole("button", { name: "移除图片 13" }));
    expect(onRemove).toHaveBeenCalledWith("13");
    await user.click(screen.getByRole("button", { name: "添加图片" }));
    expect(onUpload).toHaveBeenCalledWith("14", "校园 风景");
    await waitFor(() => expect(apiMocks.myMediaUpload).toHaveBeenCalledWith("12"));
    await expectNoAccessibilityViolations(view.container);
  });

  it("keeps removal available when owner status refresh fails", async () => {
    apiMocks.myMediaUpload.mockRejectedValueOnce(new Error("状态暂不可用"));
    const user = userEvent.setup();
    const onRemove = vi.fn();
    render(
      <ForumImageAttachments
        usage="forum_comment"
        assetIds={["99"]}
        maxImages={4}
        onUpload={vi.fn()}
        onRemove={onRemove}
      />,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent("状态暂不可用");
    await user.click(screen.getByRole("button", { name: "移除图片 99" }));
    expect(onRemove).toHaveBeenCalledWith("99");
  });
});
