import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { AppealsPanel } from "./appeals-panel";

const apiMocks = vi.hoisted(() => ({
  list: vi.fn(),
  startReview: vi.fn(),
  decide: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    adminAppeals: apiMocks.list,
    startAdminAppealReview: apiMocks.startReview,
    decideAdminAppeal: apiMocks.decide,
  },
}));

const submittedAppeal = {
  id: "51",
  governanceEventId: "44",
  originalAction: "identity.user.sanctioned",
  originalReason: "账号安全复核",
  targetKind: "sanction" as const,
  targetId: "23",
  dispositionKind: "suspend" as const,
  status: "submitted" as const,
  submissionReason: "制裁期限不成比例",
  submittedAt: 1_720_000_000,
  appealableUntil: 1_722_592_000,
  reviewStartedAt: null,
  decisionReason: null,
  amendment: null,
  decidedAt: null,
  version: 1,
  history: [
    {
      id: "61",
      fromStatus: null,
      toStatus: "submitted" as const,
      reason: "制裁期限不成比例",
      metadata: null,
      createdAt: 1_720_000_000,
    },
  ],
  appellantAccountId: "7",
  reviewerAccountId: null,
};

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <AppealsPanel />
    </QueryClientProvider>,
  );
}

describe("AppealsPanel", () => {
  beforeEach(() => {
    apiMocks.list.mockReset().mockResolvedValue({
      items: [submittedAppeal],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.startReview.mockReset().mockResolvedValue({
      ...submittedAppeal,
      status: "in_review",
      version: 2,
      reviewerAccountId: "8",
    });
    apiMocks.decide.mockReset().mockResolvedValue({
      ...submittedAppeal,
      status: "overturned",
      version: 3,
      reviewerAccountId: "8",
    });
  });

  it("requires a reason before independently claiming an appeal", async () => {
    const user = userEvent.setup();
    const view = renderPanel();

    expect(await screen.findByText("制裁期限不成比例")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "领取复核" }));
    expect(screen.getByRole("button", { name: "领取复核" })).toBeDisabled();
    await user.type(screen.getByLabelText("操作原因"), "由独立复核员处理");
    await user.click(screen.getByRole("button", { name: "领取复核" }));

    await waitFor(() => expect(apiMocks.startReview).toHaveBeenCalledWith(
      "51",
      1,
      "由独立复核员处理",
    ));
    await expectNoAccessibilityViolations(view.container);
  });

  it("submits an explicit overturn decision with optimistic versioning", async () => {
    apiMocks.list.mockResolvedValue({
      items: [{
        ...submittedAppeal,
        status: "in_review",
        version: 2,
        reviewerAccountId: "8",
      }],
      hasMore: false,
      nextCursor: null,
    });
    const user = userEvent.setup();
    renderPanel();

    await user.click(await screen.findByRole("button", { name: "撤销" }));
    await user.type(screen.getByLabelText("复核结论与理由"), "原处置不符合比例原则");
    await user.click(screen.getByRole("button", { name: "提交决定" }));

    await waitFor(() => expect(apiMocks.decide).toHaveBeenCalledWith("51", {
      expectedVersion: 2,
      outcome: "overturned",
      reason: "原处置不符合比例原则",
      amendedEndsAt: undefined,
    }));
  });
});
