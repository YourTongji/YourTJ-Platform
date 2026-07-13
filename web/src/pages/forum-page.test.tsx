import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ForumPage } from "./forum-page";

const apiMocks = vi.hoisted(() => ({
  boards: vi.fn(),
  tags: vi.fn(),
  threads: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ isAuthenticated: true }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: apiMocks,
}));

const firstThread = {
  id: "11",
  boardId: "1",
  authorHandle: "alice",
  authorDisplayName: "Alice Chen",
  authorAvatar: null,
  title: "第一页帖子",
  bodyExcerpt: "第一页摘要",
  contentVersion: 1,
  replyCount: 2,
  voteCount: 5,
  hotScore: 3,
  status: "visible" as const,
  createdAt: 1_700_000_000,
  lastActivityAt: 1_700_000_100,
  tags: [],
  canEdit: false,
  canDelete: false,
  canModerate: false,
};

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/forum?feed=new"]}>
        <ForumPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("ForumPage", () => {
  beforeEach(() => {
    apiMocks.boards.mockReset().mockResolvedValue([{
      id: "1",
      slug: "campus",
      name: "校园生活",
      description: "校园讨论",
      threadCount: 2,
      canPost: true,
      postingRestriction: "none",
      minTrustToPost: 0,
    }]);
    apiMocks.tags.mockReset().mockResolvedValue([]);
    apiMocks.threads.mockReset().mockImplementation(async ({ cursor }) => cursor
      ? {
          items: [{ ...firstThread, id: "10", title: "第二页帖子" }],
          nextCursor: null,
          hasMore: false,
        }
      : {
          items: [firstThread],
          nextCursor: "cursor-11",
          hasMore: true,
        });
  });

  it("continues the selected feed with the server cursor", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect(await screen.findByText("第一页帖子")).toBeVisible();
    expect(screen.getAllByText("Alice Chen")[0]).toBeVisible();
    expect(screen.getAllByText("@alice")[0]).toBeVisible();
    await user.click(screen.getByRole("button", { name: "加载更多帖子" }));

    expect(await screen.findByText("第二页帖子")).toBeVisible();
    await waitFor(() => expect(apiMocks.threads).toHaveBeenLastCalledWith({
      feed: "new",
      board: undefined,
      tag: undefined,
      cursor: "cursor-11",
    }));
    await expectNoAccessibilityViolations(view.container);
  });
});
