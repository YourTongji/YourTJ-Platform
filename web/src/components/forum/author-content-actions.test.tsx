import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api/client";
import type { Comment, ThreadDetailWithPoll } from "@/lib/api/types";
import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { CommentAuthorActions, ThreadAuthorActions } from "./author-content-actions";

const apiMocks = vi.hoisted(() => ({
  comments: vi.fn(),
  deleteComment: vi.fn(),
  deleteThread: vi.fn(),
  thread: vi.fn(),
  updateComment: vi.fn(),
  updateThread: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/components/content/markdown-editor", () => ({
  MarkdownEditor: ({
    value,
    onChange,
    label,
    maxLength,
  }: {
    value: string;
    onChange: (value: string) => void;
    label: string;
    maxLength: number;
  }) => (
    <textarea
      aria-label={label}
      value={value}
      maxLength={maxLength}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

const thread: ThreadDetailWithPoll = {
  id: "42",
  boardId: "1",
  authorHandle: "alice",
  authorAvatar: null,
  authorId: "1",
  title: "原始标题",
  body: "原始正文",
  contentFormat: "markdown_v1",
  contentVersion: 2,
  replyCount: 0,
  voteCount: 0,
  hotScore: null,
  tags: [],
  attachments: [],
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
  canEdit: true,
  canDelete: true,
  canModerate: false,
};

const comment: Comment = {
  id: "7",
  threadId: "42",
  parentId: null,
  path: "0001",
  authorHandle: "bob",
  authorAvatar: null,
  authorId: "2",
  body: "原始回复",
  contentFormat: "markdown_v1",
  contentVersion: 4,
  attachments: [],
  voteCount: 0,
  viewerVote: null,
  isBookmarked: false,
  isDeleted: false,
  isHidden: false,
  editedAt: null,
  createdAt: 1_700_000_010,
  quotedCommentId: null,
  isSolved: false,
  canEdit: true,
  canDelete: true,
  canModerate: false,
};

function renderActions(node: React.ReactNode) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>{node}</MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("author content actions", () => {
  beforeEach(() => {
    for (const mock of Object.values(apiMocks)) mock.mockReset();
  });

  it("preserves a local thread edit through a version conflict and retries against the latest version", async () => {
    const user = userEvent.setup();
    apiMocks.updateThread
      .mockRejectedValueOnce(new ApiError(409, "content changed", "VERSION_CONFLICT", { currentVersion: 3 }))
      .mockResolvedValueOnce({ ...thread, title: "我的标题", contentVersion: 4 });
    apiMocks.thread.mockResolvedValue({ ...thread, title: "线上标题", contentVersion: 3 });
    const view = renderActions(<ThreadAuthorActions thread={thread} />);

    await user.click(screen.getByRole("button", { name: "编辑" }));
    const title = screen.getByLabelText("标题");
    await user.clear(title);
    await user.type(title, "我的标题");
    await user.click(screen.getByRole("button", { name: "保存修改" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("你的输入仍保留");
    expect(title).toHaveValue("我的标题");
    await expectNoAccessibilityViolations(view.baseElement);

    await user.click(screen.getByRole("button", { name: "保留我的内容并重试" }));
    await waitFor(() => expect(apiMocks.updateThread).toHaveBeenCalledTimes(2));
    expect(apiMocks.updateThread).toHaveBeenLastCalledWith("42", expect.objectContaining({
      expectedVersion: 3,
      title: "我的标题",
    }));
  });

  it("keeps a comment draft visible when the canonical reply changes", async () => {
    const user = userEvent.setup();
    apiMocks.updateComment.mockRejectedValue(
      new ApiError(409, "content changed", "VERSION_CONFLICT", { currentVersion: 5 }),
    );
    apiMocks.comments.mockResolvedValue({
      items: [{ ...comment, body: "线上回复", contentVersion: 5 }],
      hasMore: false,
      nextCursor: null,
    });
    const view = renderActions(<CommentAuthorActions comment={comment} threadId="42" />);

    await user.click(screen.getByRole("button", { name: "编辑" }));
    const body = screen.getByLabelText("回复正文");
    await user.clear(body);
    await user.type(body, "我的本地回复");
    await user.click(screen.getByRole("button", { name: "保存修改" }));

    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(body).toHaveValue("我的本地回复");
    expect(screen.getByRole("button", { name: "载入线上版本" })).toBeEnabled();
    await expectNoAccessibilityViolations(view.baseElement);
  });
});
