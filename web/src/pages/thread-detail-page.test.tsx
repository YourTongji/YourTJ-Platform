import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ThreadDetailPage } from "./thread-detail-page";

const apiMocks = vi.hoisted(() => ({
  thread: vi.fn(),
  boards: vi.fn(),
  comments: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ account: null, isAuthenticated: false }),
}));

vi.mock("@/components/forum/moderation-controls", () => ({
  CommentModerationControls: () => null,
  ThreadModerationControls: () => null,
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    thread: apiMocks.thread,
    boards: apiMocks.boards,
    comments: apiMocks.comments,
  },
}));

function renderPage() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/forum/threads/42"]}>
        <Routes>
          <Route path="/forum/threads/:id" element={<ThreadDetailPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("ThreadDetailPage Markdown content", () => {
  beforeEach(() => {
    apiMocks.boards.mockReset().mockResolvedValue([{ id: "1", name: "校园生活" }]);
    apiMocks.thread.mockReset().mockResolvedValue({
      id: "42",
      boardId: "1",
      authorHandle: "alice",
      authorId: "1",
      title: "格式化讨论",
      body: "欢迎阅读 **重要内容**。",
      contentFormat: "markdown_v1",
      replyCount: 1,
      voteCount: 2,
      hotScore: 1,
      tags: [],
      status: "visible",
      pinnedAt: null,
      pinnedGlobally: false,
      featuredAt: null,
      closedAt: null,
      archivedAt: null,
      deletedAt: null,
      editedAt: null,
      hiddenAt: null,
      createdAt: 1_700_000_000,
      lastActivityAt: 1_700_000_000,
      solvedAnswerId: null,
      viewerVote: null,
      isBookmarked: false,
      myLastReadCommentId: null,
      mySubscriptionLevel: null,
      poll: null,
    });
    apiMocks.comments.mockReset().mockResolvedValue({
      items: [{
        id: "7",
        threadId: "42",
        parentId: null,
        path: "0001",
        authorHandle: "bob",
        authorId: "2",
        body: "回复也支持 `code`。",
        contentFormat: "markdown_v1",
        voteCount: 0,
        viewerVote: null,
        isBookmarked: false,
        isDeleted: false,
        isHidden: false,
        editedAt: null,
        createdAt: 1_700_000_010,
        quotedCommentId: null,
        isSolved: false,
      }],
      hasMore: false,
      nextCursor: null,
    });
  });

  it("renders persisted Markdown formats instead of exposing source markers", async () => {
    const view = renderPage();

    expect(await screen.findByRole("strong")).toHaveTextContent("重要内容");
    expect(screen.getByText("code").tagName).toBe("CODE");
    expect(screen.queryByText(/\*\*重要内容\*\*/)).not.toBeInTheDocument();
    await expectNoAccessibilityViolations(view.container);
  });
});
