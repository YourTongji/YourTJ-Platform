import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as React from "react";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { MarkdownEditor } from "./markdown-editor";

const apiMocks = vi.hoisted(() => ({ myMediaUpload: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/components/media/media-upload-button", () => ({
  MediaUploadButton: ({ onUploaded }: { onUploaded: (upload: {
    uploadId: string;
    ossKey: string;
    status: "pending";
    originalName: string;
  }) => void }) => (
    <button
      type="button"
      onClick={() => onUploaded({
        uploadId: "42",
        ossKey: "must-not-render",
        status: "pending",
        originalName: "[校园]\n风景.png",
      })}
    >
      添加图片
    </button>
  ),
}));

describe("MarkdownEditor", () => {
  it("uses the same safe renderer for its preview", async () => {
    const user = userEvent.setup();
    const view = render(
      <MarkdownEditor
        value="**可预览内容**"
        onChange={vi.fn()}
        label="主题正文"
        maxLength={50_000}
      />,
    );

    expect(screen.getByLabelText("主题正文")).toBeVisible();
    await user.click(screen.getByRole("tab", { name: "预览" }));
    expect(screen.getByText("可预览内容").tagName).toBe("STRONG");
    await expectNoAccessibilityViolations(view.container);
  });

  it("inserts and removes a canonical platform image reference without exposing an object key", async () => {
    apiMocks.myMediaUpload.mockResolvedValue({
      id: "42",
      kind: "image",
      usage: "forum_thread",
      bytes: 100,
      mime: "image/png",
      status: "pending",
      imageWidth: null,
      imageHeight: null,
      createdAt: 1_700_000_000,
    });
    const user = userEvent.setup();

    function Harness() {
      const [value, setValue] = React.useState("");
      const [assetIds, setAssetIds] = React.useState<string[]>([]);
      return (
        <>
          <MarkdownEditor
            value={value}
            onChange={setValue}
            label="主题正文"
            maxLength={50_000}
            attachmentUsage="forum_thread"
            attachmentAssetIds={assetIds}
            onAttachmentAssetIdsChange={setAssetIds}
            maxImages={8}
          />
          <output data-testid="source">{value}</output>
          <output data-testid="asset-ids">{assetIds.join(",")}</output>
        </>
      );
    }

    const view = render(<Harness />);
    await user.click(screen.getByRole("button", { name: "添加图片" }));
    expect(screen.getByTestId("source")).toHaveTextContent("![校园 风景](yourtj-asset:42)");
    expect(screen.getByTestId("asset-ids")).toHaveTextContent("42");
    expect(view.container).not.toHaveTextContent("must-not-render");

    await user.click(await screen.findByRole("button", { name: "移除图片 42" }));
    expect(screen.getByTestId("source")).toBeEmptyDOMElement();
    expect(screen.getByTestId("asset-ids")).toBeEmptyDOMElement();
    await expectNoAccessibilityViolations(view.container);
  });
});
