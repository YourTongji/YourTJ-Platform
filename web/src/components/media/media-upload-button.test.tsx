import { fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { MediaUploadButton } from "./media-upload-button";

const apiMocks = vi.hoisted(() => ({ mediaUploadCredentials: vi.fn() }));
const toastMocks = vi.hoisted(() => ({ error: vi.fn(), success: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("sonner", () => ({ toast: toastMocks }));

describe("MediaUploadButton", () => {
  beforeEach(() => {
    apiMocks.mediaUploadCredentials.mockReset();
    toastMocks.error.mockReset();
    toastMocks.success.mockReset();
  });

  it("does not advertise GIF and rejects one that bypasses the file picker", async () => {
    const onUploaded = vi.fn();
    const view = render(<MediaUploadButton kind="image" onUploaded={onUploaded} />);
    const input = view.container.querySelector<HTMLInputElement>('input[type="file"]');

    expect(input).not.toBeNull();
    expect(input).toHaveAttribute("accept", "image/jpeg,image/png,image/webp");
    fireEvent.change(input!, {
      target: {
        files: [new File(["GIF89a"], "animated.gif", { type: "image/gif" })],
      },
    });

    await waitFor(() => expect(toastMocks.error).toHaveBeenCalledWith(
      "仅支持静态 JPEG、PNG 或 WebP 图片；GIF 或其他动图请转换为静态图片后重新上传",
    ));
    expect(apiMocks.mediaUploadCredentials).not.toHaveBeenCalled();
    expect(onUploaded).not.toHaveBeenCalled();
  });
});
