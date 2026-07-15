import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { OneboxPreviewDialog } from "./onebox-preview-dialog";

const apiMocks = vi.hoisted(() => ({ onebox: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

describe("OneboxPreviewDialog", () => {
  beforeEach(() => {
    apiMocks.onebox.mockReset();
  });

  it("only requests a preview after explicit consent and never renders a remote image", async () => {
    apiMocks.onebox.mockResolvedValue({
      type: "card",
      url: "https://news.tongji.edu.cn/article",
      title: "校园新闻",
      description: "一段公开文字摘要",
      imageUrl: "https://remote.example/tracker.png",
      siteName: "同济新闻网",
    });
    const onInsert = vi.fn();
    const user = userEvent.setup();
    const view = render(<OneboxPreviewDialog onInsert={onInsert} />);

    await user.click(screen.getByRole("button", { name: "预览并插入链接" }));
    await user.type(screen.getByRole("textbox", { name: "HTTPS 链接" }), "https://news.tongji.edu.cn/article");
    expect(apiMocks.onebox).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "安全预览" }));
    expect(await screen.findByText("校园新闻")).toBeVisible();
    expect(screen.getByText("一段公开文字摘要")).toBeVisible();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(apiMocks.onebox).toHaveBeenCalledWith(
      "https://news.tongji.edu.cn/article",
      expect.any(AbortSignal),
    );

    await user.click(screen.getByRole("button", { name: "插入预览链接" }));
    expect(onInsert).toHaveBeenCalledWith("https://news.tongji.edu.cn/article", "校园新闻");
    await expectNoAccessibilityViolations(view.container);
  });

  it("rejects non-HTTPS input before calling the server", async () => {
    const user = userEvent.setup();
    render(<OneboxPreviewDialog onInsert={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "预览并插入链接" }));
    await user.type(screen.getByRole("textbox", { name: "HTTPS 链接" }), "http://example.com");
    await user.click(screen.getByRole("button", { name: "安全预览" }));

    expect(screen.getByRole("alert")).toHaveTextContent("只支持使用标准 HTTPS 端口的链接");
    expect(apiMocks.onebox).not.toHaveBeenCalled();
  });

  it("aborts an obsolete preview and never lets its response replace the current URL", async () => {
    let resolveObsoletePreview: ((value: {
      type: "card";
      url: string;
      title: string;
      description: null;
      imageUrl: null;
      siteName: string;
    }) => void) | undefined;
    apiMocks.onebox.mockImplementation((url: string) => {
      if (url === "https://old.example/article") {
        return new Promise((resolve) => {
          resolveObsoletePreview = resolve;
        });
      }
      return Promise.resolve({
        type: "card",
        url: "https://new.example/article",
        title: "新链接",
        description: null,
        imageUrl: null,
        siteName: "New site",
      });
    });
    const onInsert = vi.fn();
    const user = userEvent.setup();
    render(<OneboxPreviewDialog onInsert={onInsert} />);

    await user.click(screen.getByRole("button", { name: "预览并插入链接" }));
    const input = screen.getByRole("textbox", { name: "HTTPS 链接" });
    await user.type(input, "https://old.example/article");
    await user.click(screen.getByRole("button", { name: "安全预览" }));
    const obsoleteSignal = apiMocks.onebox.mock.calls[0]?.[1] as AbortSignal;

    await user.clear(input);
    await user.type(input, "https://new.example/article");
    expect(obsoleteSignal.aborted).toBe(true);
    await user.click(screen.getByRole("button", { name: "安全预览" }));
    expect(await screen.findByText("新链接")).toBeVisible();

    await act(async () => {
      resolveObsoletePreview?.({
        type: "card",
        url: "https://old.example/article",
        title: "旧链接",
        description: null,
        imageUrl: null,
        siteName: "Old site",
      });
      await Promise.resolve();
    });

    expect(screen.queryByText("旧链接")).not.toBeInTheDocument();
    expect(screen.getByText("新链接")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "插入预览链接" }));
    expect(onInsert).toHaveBeenCalledWith("https://new.example/article", "新链接");
  });
});
