import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

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
  afterEach(() => {
    Reflect.deleteProperty(navigator, "share");
    Reflect.deleteProperty(navigator, "clipboard");
  });

  beforeEach(() => {
    apiMocks.boards.mockReset().mockResolvedValue([{ id: "1", name: "校园生活" }]);
    apiMocks.thread.mockReset().mockResolvedValue({
      id: "42",
      boardId: "1",
      authorHandle: "alice",
      authorDisplayName: "Alice Chen",
      authorAvatar: {
        assetId: "11",
        variant: "thumb_256",
        url: "https://media.example.test/alice.webp",
        expiresAt: Math.floor(Date.now() / 1000) + 300,
        mime: "image/webp",
        width: 256,
        height: 256,
      },
      authorId: "1",
      title: "格式化讨论",
      body: "欢迎阅读 **重要内容**。\n\n![校园日景](yourtj-asset:21)\n\n![校园夜景](yourtj-asset:22)",
      contentFormat: "markdown_v1",
      attachments: [
        {
          assetId: "21",
          reference: "yourtj-asset:21",
          position: 0,
          alt: "校园日景",
          url: "https://media.example.test/day.webp",
          expiresAt: Math.floor(Date.now() / 1000) + 300,
          width: 1280,
          height: 720,
        },
        {
          assetId: "22",
          reference: "yourtj-asset:22",
          position: 1,
          alt: "校园夜景",
          url: "https://media.example.test/night.webp",
          expiresAt: Math.floor(Date.now() / 1000) + 300,
          width: 1280,
          height: 720,
        },
      ],
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
        authorDisplayName: "Bob Li",
        authorAvatar: {
          assetId: "12",
          variant: "thumb_256",
          url: "https://media.example.test/bob.webp",
          expiresAt: Math.floor(Date.now() / 1000) + 300,
          mime: "image/webp",
          width: 256,
          height: 256,
        },
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
    expect(screen.getByRole("link", { name: "Alice Chen" })).toHaveAttribute(
      "href",
      "/profile/alice",
    );
    expect(screen.getByRole("link", { name: "查看 @alice 的个人主页" })).toHaveAttribute(
      "href",
      "/profile/alice",
    );
    expect(screen.getByRole("link", { name: "查看 @bob 的个人主页" })).toHaveAttribute(
      "href",
      "/profile/bob",
    );
    expect(screen.getByText("Bob Li")).toBeVisible();
    expect(screen.getByText("@bob")).toBeVisible();
    expect(screen.getByText("code").tagName).toBe("CODE");
    expect(screen.queryByText(/\*\*重要内容\*\*/)).not.toBeInTheDocument();
    await expectNoAccessibilityViolations(view.container);
  });

  it("navigates all clean thread attachments in one lightbox gallery", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "查看大图：校园日景" }));
    const dialog = screen.getByRole("dialog", { name: /校园日景/ });
    expect(dialog).toBeVisible();
    await user.click(screen.getByRole("button", { name: "下一张图片" }));
    expect(within(dialog).getByRole("img", { name: "校园夜景" })).toBeVisible();
  });

  it("shares the canonical thread deep link", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    renderPage();

    await user.click(await screen.findByRole("button", { name: "分享" }));

    expect(writeText).toHaveBeenCalledWith(
      new URL("/forum/threads/42", window.location.origin).toString(),
    );
  });
});
