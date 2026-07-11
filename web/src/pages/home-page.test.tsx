import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { HomePage } from "./home-page";

const apiMocks = vi.hoisted(() => ({
  announcements: vi.fn(),
  threads: vi.fn(),
}));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ account: null }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    announcements: apiMocks.announcements,
    threads: apiMocks.threads,
    myActivity: vi.fn(),
  },
}));

const firstThread = {
  id: "21",
  boardId: "1",
  authorHandle: "alice",
  title: "首页第一页",
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
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("HomePage", () => {
  beforeEach(() => {
    apiMocks.announcements.mockReset().mockResolvedValue([]);
    apiMocks.threads.mockReset().mockImplementation(async ({ cursor }) => cursor
      ? {
          items: [{ ...firstThread, id: "20", title: "首页第二页" }],
          nextCursor: null,
          hasMore: false,
        }
      : {
          items: [firstThread],
          nextCursor: "cursor-21",
          hasMore: true,
        });
  });

  it("continues the active home feed with the server cursor", async () => {
    const user = userEvent.setup();
    renderPage();

    expect((await screen.findAllByText("首页第一页"))[0]).toBeVisible();
    await user.click(screen.getByRole("button", { name: "加载更多动态" }));

    expect((await screen.findAllByText("首页第二页"))[0]).toBeVisible();
    await waitFor(() => expect(apiMocks.threads).toHaveBeenLastCalledWith({
      feed: "hot",
      cursor: "cursor-21",
    }));
  });
});
