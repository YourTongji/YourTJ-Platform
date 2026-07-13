import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { CommunityFeed } from "./community-feed";

vi.mock("@/components/ui/avatar", () => ({
  Avatar: ({ children, ...props }: ComponentProps<"span">) => <span {...props}>{children}</span>,
  AvatarImage: ({
    onLoadingStatusChange,
    ...props
  }: ComponentProps<"img"> & { onLoadingStatusChange?: (status: string) => void }) => (
    <img {...props} onError={() => onLoadingStatusChange?.("error")} />
  ),
  AvatarFallback: ({ children, ...props }: ComponentProps<"span">) => (
    <span {...props}>{children}</span>
  ),
}));

const thread = {
  id: "42",
  boardId: "1",
  authorHandle: "alice",
  authorDisplayName: "Alice Chen",
  authorAvatar: {
    assetId: "7",
    variant: "thumb_256" as const,
    url: "https://media.example.test/alice.webp",
    expiresAt: Math.floor(Date.now() / 1000) + 300,
    mime: "image/webp" as const,
    width: 256,
    height: 256,
  },
  title: "关注动态",
  bodyExcerpt: "这是从服务端正文投影生成的摘要。",
  contentVersion: 1,
  replyCount: 3,
  voteCount: 8,
  hotScore: 2,
  status: "visible" as const,
  createdAt: 1_700_000_000,
  lastActivityAt: 1_700_000_100,
  tags: ["campus"],
  viewerVote: "up" as const,
  isBookmarked: true,
  attachments: [],
  canEdit: false,
  canDelete: false,
  canModerate: false,
};

describe("CommunityFeed", () => {
  it("offers the canonical following feed and renders accessible server summaries", async () => {
    const onModeChange = vi.fn();
    const user = userEvent.setup();
    const view = render(
      <MemoryRouter>
        <CommunityFeed
          mode="hot"
          onModeChange={onModeChange}
          items={[thread]}
          isLoading={false}
          onRetry={vi.fn()}
          isAuthenticated
          onAttachmentDeliveryRefresh={vi.fn()}
        />
      </MemoryRouter>,
    );

    await user.click(screen.getByRole("tab", { name: "关注" }));
    expect(onModeChange).toHaveBeenCalledWith("following");
    expect(screen.getByText("这是从服务端正文投影生成的摘要。")).toBeVisible();
    expect(screen.getByRole("link", { name: /关注动态/ })).toHaveAttribute(
      "href",
      "/forum/threads/42",
    );
    expect(screen.getByAltText("alice 的头像")).toHaveAttribute(
      "src",
      "https://media.example.test/alice.webp",
    );
    expect(screen.getByAltText("alice 的头像")).toHaveAttribute("loading", "lazy");
    expect(screen.getByAltText("alice 的头像")).toHaveAttribute("decoding", "async");
    await expectNoAccessibilityViolations(view.container);
  });

  it("refreshes the owning feed when an author avatar delivery fails", () => {
    const onDeliveryRefresh = vi.fn();
    render(
      <MemoryRouter>
        <CommunityFeed
          mode="hot"
          onModeChange={vi.fn()}
          items={[thread]}
          isLoading={false}
          onRetry={vi.fn()}
          isAuthenticated
          onAttachmentDeliveryRefresh={onDeliveryRefresh}
        />
      </MemoryRouter>,
    );

    fireEvent.error(screen.getByAltText("alice 的头像"));
    expect(onDeliveryRefresh).toHaveBeenCalledOnce();
  });

  it("keeps the following feed unavailable to anonymous visitors", () => {
    render(
      <MemoryRouter>
        <CommunityFeed
          mode="hot"
          onModeChange={vi.fn()}
          items={[]}
          isLoading={false}
          onRetry={vi.fn()}
          isAuthenticated={false}
          onAttachmentDeliveryRefresh={vi.fn()}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole("tab", { name: "关注" })).toBeDisabled();
  });

  it("offers an explicit accessible control for the next cursor page", async () => {
    const onLoadMore = vi.fn();
    const user = userEvent.setup();

    render(
      <MemoryRouter>
        <CommunityFeed
          mode="new"
          onModeChange={vi.fn()}
          items={[thread]}
          isLoading={false}
          onRetry={vi.fn()}
          hasMore
          isLoadingMore={false}
          onLoadMore={onLoadMore}
          isAuthenticated
          onAttachmentDeliveryRefresh={vi.fn()}
        />
      </MemoryRouter>,
    );

    await user.click(screen.getByRole("button", { name: "加载更多动态" }));
    expect(onLoadMore).toHaveBeenCalledOnce();
  });
});
