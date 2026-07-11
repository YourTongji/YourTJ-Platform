import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { CommunityFeed } from "./community-feed";

const thread = {
  id: "42",
  boardId: "1",
  authorHandle: "alice",
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
    await expectNoAccessibilityViolations(view.container);
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
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole("tab", { name: "关注" })).toBeDisabled();
  });
});
