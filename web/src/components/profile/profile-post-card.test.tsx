import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ProfilePostCard } from "./profile-post-card";

describe("ProfilePostCard", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(navigator, "share");
    Reflect.deleteProperty(navigator, "clipboard");
  });

  it("exposes real counts, navigation, bookmarking, and native sharing", async () => {
    const user = userEvent.setup();
    const onToggleBookmark = vi.fn();
    const share = vi.fn().mockResolvedValue(undefined);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const view = render(
      <MemoryRouter>
        <ProfilePostCard
          authorName="Alice"
          authorHandle="alice"
          post={{
            id: "thread-1",
            title: "真实主题",
            body: "正文摘要",
            createdAtLabel: "刚刚",
            replyCount: 12,
            voteCount: 34,
            href: "/forum/threads/thread-1",
            isBookmarked: false,
          }}
          onToggleBookmark={onToggleBookmark}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole("link", { name: /真实主题/ })).toHaveAttribute(
      "href",
      "/forum/threads/thread-1",
    );
    expect(screen.getByLabelText("12 条回复")).toBeVisible();
    expect(screen.getByLabelText("34 个赞")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "收藏" }));
    expect(onToggleBookmark).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: "分享" }));
    expect(share).toHaveBeenCalledWith({
      title: "真实主题",
      url: new URL("/forum/threads/thread-1", window.location.origin).toString(),
    });

    await user.click(screen.getByRole("button", { name: "更多操作" }));
    expect(await screen.findByRole("menuitem", { name: "打开内容" })).toBeVisible();
    await user.click(screen.getByRole("menuitem", { name: "复制链接" }));
    expect(writeText).toHaveBeenCalledWith(
      new URL("/forum/threads/thread-1", window.location.origin).toString(),
    );
    await expectNoAccessibilityViolations(view.container);
  });

  it("keeps attachment preview outside navigation and opens it in a lightbox", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <ProfilePostCard
          authorName="Alice"
          authorHandle="alice"
          post={{
            id: "thread-1",
            title: "带图主题",
            createdAtLabel: "刚刚",
            href: "/forum/threads/thread-1",
            attachment: {
              assetId: "asset-1",
              reference: "yourtj-asset:asset-1",
              position: 0,
              alt: "校园樱花",
              url: "https://media.example.test/cherry.webp",
              expiresAt: Math.floor(Date.now() / 1000) + 300,
              width: 1280,
              height: 720,
            },
          }}
        />
      </MemoryRouter>,
    );

    const contentLink = screen.getByRole("link", { name: "带图主题" });
    expect(contentLink.querySelector("button")).toBeNull();
    await user.click(screen.getByRole("button", { name: "查看大图：校园樱花" }));
    expect(screen.getByRole("dialog", { name: "校园樱花" })).toBeVisible();
  });
});
