import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { SearchDialog } from "./search-dialog";

const searchMock = vi.hoisted(() => vi.fn());
const avatarMock = vi.hoisted(() => ({
  onLoadingStatusChange: undefined as undefined | ((status: "error") => void),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: { search: searchMock },
}));

vi.mock("@/components/ui/avatar", () => ({
  Avatar: ({ children }: PropsWithChildren) => <div>{children}</div>,
  AvatarFallback: ({ children }: PropsWithChildren) => <span>{children}</span>,
  AvatarImage: ({
    onLoadingStatusChange,
    src,
  }: { onLoadingStatusChange?: (status: "error") => void; src?: string }) => {
    avatarMock.onLoadingStatusChange = onLoadingStatusChange;
    return <span data-testid="signed-avatar" data-src={src} />;
  },
}));

describe("SearchDialog", () => {
  beforeEach(() => {
    avatarMock.onLoadingStatusChange = undefined;
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
      users: [
        {
          id: "41",
          handle: "alice",
          displayName: "Alice",
          avatarUrl: null,
          role: "user",
          followerCount: 12,
          following: true,
        },
      ],
      boards: [
        {
          id: "51",
          slug: "study",
          name: "学习交流",
          description: null,
          threadCount: 20,
        },
      ],
      tags: [
        {
          id: "61",
          slug: "algorithm",
          name: "算法",
          description: null,
          threadCount: 8,
        },
      ],
    });
  });

  it("renders typed course, review, and thread destinations", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <SearchDialog open onOpenChange={onOpenChange} />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.type(screen.getByRole("textbox", { name: "搜索关键词" }), "数据");

    expect(await screen.findByText("数据结构复习资料")).toBeVisible();
    expect(screen.getByRole("link", { name: /CS101/ })).toHaveAttribute("href", "/courses/12");
    expect(screen.getByRole("link", { name: /讲解清晰/ })).toHaveAttribute(
      "href",
      "/courses/12?review=21#review-21",
    );
    expect(screen.getByRole("link", { name: /数据结构复习资料/ })).toHaveAttribute(
      "href",
      "/forum/threads/31",
    );
    expect(screen.getByRole("link", { name: /Alice/ })).toHaveAttribute("href", "/profile/alice");
    expect(screen.getByRole("link", { name: "学习交流" })).toHaveAttribute("href", "/forum?board=51");
    expect(screen.getByRole("link", { name: "#算法" })).toHaveAttribute("href", "/forum?tag=algorithm");
    await expectNoAccessibilityViolations(document.body);

    const input = screen.getByRole("textbox", { name: "搜索关键词" });
    input.focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("link", { name: /CS101/ })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("link", { name: /讲解清晰/ })).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("debounces input and aborts an obsolete request", async () => {
    const response = await searchMock();
    searchMock.mockClear();
    const signals: AbortSignal[] = [];
    searchMock.mockImplementation((...parameters: unknown[]) => {
      const query = parameters[0];
      const signal = parameters[4] as AbortSignal;
      signals.push(signal);
      if (query === "数据结构") return Promise.resolve(response);
      return new Promise((_, reject) => {
        signal.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")));
      });
    });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <SearchDialog open onOpenChange={vi.fn()} />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    const input = screen.getByRole("textbox", { name: "搜索关键词" });
    await user.type(input, "数据");
    expect(searchMock).not.toHaveBeenCalled();
    await waitFor(() => expect(searchMock).toHaveBeenCalledTimes(1));
    await user.type(input, "结构");
    await waitFor(() => expect(searchMock).toHaveBeenCalledTimes(2));

    expect(signals[0]?.aborted).toBe(true);
    expect(signals[1]?.aborted).toBe(false);
    expect(await screen.findByText("数据结构复习资料")).toBeVisible();
  });

  it("closes from the search field with Escape", async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const onOpenChange = vi.fn();
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <SearchDialog open onOpenChange={onOpenChange} />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.click(screen.getByRole("textbox", { name: "搜索关键词" }));
    await user.keyboard("{Escape}");

    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("opens the full search route from Enter without waiting for the preview request", async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const onOpenChange = vi.fn();
    const user = userEvent.setup();
    function LocationProbe() {
      const location = useLocation();
      return <output data-testid="location">{location.pathname}{location.search}</output>;
    }
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <SearchDialog open onOpenChange={onOpenChange} />
          <LocationProbe />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.type(screen.getByRole("textbox", { name: "搜索关键词" }), "数据{Enter}");

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(screen.getByTestId("location")).toHaveTextContent("/search?q=%E6%95%B0%E6%8D%AE");
    expect(searchMock).not.toHaveBeenCalled();
  });

  it("refetches the owning search result when a signed avatar expires", async () => {
    const response = await searchMock();
    response.users[0].avatarUrl = "https://cdn.example/avatar.webp?auth_key=old";
    searchMock.mockClear();
    searchMock.mockResolvedValue(response);
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <SearchDialog open onOpenChange={vi.fn()} />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.type(screen.getByRole("textbox", { name: "搜索关键词" }), "数据");
    expect(await screen.findByTestId("signed-avatar")).toHaveAttribute(
      "data-src",
      "https://cdn.example/avatar.webp?auth_key=old",
    );
    act(() => {
      avatarMock.onLoadingStatusChange?.("error");
    });

    await waitFor(() => expect(searchMock).toHaveBeenCalledTimes(2));
  });
});
