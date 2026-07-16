import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { PaginatedListState } from "./paginated-list-state";

describe("PaginatedListState", () => {
  it("keeps cursor pagination explicit and accessible", async () => {
    const onLoadMore = vi.fn();
    const user = userEvent.setup();
    const view = render(
      <PaginatedListState
        isLoading={false}
        isEmpty={false}
        onRetry={vi.fn()}
        hasMore
        onLoadMore={onLoadMore}
        loadMoreLabel="加载较早内容"
      >
        <article>第一批内容</article>
      </PaginatedListState>,
    );

    await user.click(screen.getByRole("button", { name: "加载较早内容" }));
    expect(onLoadMore).toHaveBeenCalledOnce();
    await expectNoAccessibilityViolations(view.container);
  });

  it("announces loading-more state and prevents duplicate requests", () => {
    render(
      <PaginatedListState
        isLoading={false}
        isEmpty={false}
        onRetry={vi.fn()}
        hasMore
        isLoadingMore
        onLoadMore={vi.fn()}
      >
        <article>第一批内容</article>
      </PaginatedListState>,
    );

    expect(screen.getByRole("button", { name: "正在加载" })).toBeDisabled();
  });
});
