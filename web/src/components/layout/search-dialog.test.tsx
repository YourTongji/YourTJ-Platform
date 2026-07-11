import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { SearchDialog } from "./search-dialog";

const searchMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/api/endpoints", () => ({
  api: { search: searchMock },
}));

describe("SearchDialog", () => {
  beforeEach(() => {
    searchMock.mockReset().mockResolvedValue({
      courses: [
        {
          id: "12",
          code: "CS101",
          name: "数据结构",
          credit: 4,
          department: "计算机系",
          teacherName: "张老师",
          reviewCount: 20,
          reviewAvg: 4.6,
        },
      ],
      reviews: [
        {
          id: "21",
          courseId: "12",
          courseName: "数据结构",
          rating: 5,
          comment: "讲解清晰",
          approveCount: 8,
          createdAt: 1_700_000_000,
        },
      ],
      threads: [
        {
          id: "31",
          title: "数据结构复习资料",
          bodyExcerpt: "整理了一份期末复习提纲",
          board: "study",
          tags: ["资料"],
          authorHandle: "alice",
          replyCount: 3,
          voteCount: 7,
          createdAt: 1_700_000_000,
          status: "visible",
        },
      ],
    });
  });

  it("renders typed course, review, and thread destinations", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <SearchDialog open onOpenChange={vi.fn()} />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.type(screen.getByRole("textbox", { name: "搜索关键词" }), "数据");

    expect(await screen.findByText("数据结构复习资料")).toBeVisible();
    expect(screen.getByRole("link", { name: /CS101/ })).toHaveAttribute("href", "/courses/12");
    expect(screen.getByRole("link", { name: /讲解清晰/ })).toHaveAttribute("href", "/courses/12");
    expect(screen.getByRole("link", { name: /数据结构复习资料/ })).toHaveAttribute(
      "href",
      "/forum/threads/31",
    );
    await expectNoAccessibilityViolations(document.body);
  });
});
