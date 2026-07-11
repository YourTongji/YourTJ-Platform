import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { SearchPage } from "./search-page";

const apiMocks = vi.hoisted(() => ({ search: vi.fn() }));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/search?q=算法"]}>
        <SearchPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("SearchPage", () => {
  beforeEach(() => {
    apiMocks.search.mockReset().mockImplementation(async (_query: string, scope: string) => ({
      courses: scope === "all" || scope === "course" ? [{
        id: "1",
        code: "CS101",
        name: "算法设计",
        credit: 3,
        department: "计算机学院",
        teacherName: "张老师",
        reviewCount: 12,
        reviewAvg: 4.8,
      }] : [],
      reviews: scope === "all" || scope === "review" ? [{
        id: "2",
        courseId: "1",
        courseName: "算法设计",
        rating: 5,
        comment: "讲解清晰",
        approveCount: 6,
        createdAt: 1_700_000_000,
      }] : [],
      threads: scope === "all" || scope === "thread" ? [{
        id: "3",
        title: "算法作业讨论",
        bodyExcerpt: "一起梳理动态规划",
        board: "study",
        tags: [],
        authorHandle: "alice",
        replyCount: 4,
        voteCount: 8,
        createdAt: 1_700_000_000,
        status: "visible",
      }] : [],
    }));
  });

  it("renders typed canonical links and applies a search scope", async () => {
    const user = userEvent.setup();
    const view = renderPage();

    expect((await screen.findAllByRole("link", { name: /算法设计/ }))[0]).toHaveAttribute("href", "/courses/1");
    expect(screen.getByRole("link", { name: /算法作业讨论/ })).toHaveAttribute("href", "/forum/threads/3");

    await user.click(screen.getByRole("button", { name: "社区帖子" }));
    await waitFor(() => expect(apiMocks.search).toHaveBeenLastCalledWith("算法", "thread", 30));
    expect(screen.queryByText("讲解清晰")).not.toBeInTheDocument();
    await expectNoAccessibilityViolations(view.container);
  });
});
