import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { PromotionsPanel } from "./promotions-panel";

const apiMocks = vi.hoisted(() => ({
  adminPromotions: vi.fn(),
  adminPromotionMetrics: vi.fn(),
  archiveAdminPromotion: vi.fn(),
  createAdminPromotion: vi.fn(),
  updateAdminPromotion: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <PromotionsPanel />
    </QueryClientProvider>,
  );
}

describe("PromotionsPanel", () => {
  beforeEach(() => {
    apiMocks.adminPromotions.mockReset().mockResolvedValue({
      items: [{
        id: "18",
        placement: "home-left-primary",
        title: "新生校园指南",
        body: "从校内资源开始认识社区。",
        ctaLabel: "查看指南",
        targetUrl: "/forum/threads/8",
        assetId: null,
        status: "published",
        effectiveState: "active",
        priority: 10,
        audience: "all",
        version: 2,
        startsAt: null,
        endsAt: null,
        archivedAt: null,
        createdAt: 1_700_000_000,
        updatedAt: 1_700_000_000,
        trackingToken: null,
        metrics: {
          from: "2026-06-12",
          to: "2026-07-11",
          impressions: 200,
          clicks: 25,
        },
      }],
      hasMore: false,
      nextCursor: null,
    });
    apiMocks.adminPromotionMetrics.mockReset().mockResolvedValue({
      summary: {
        from: "2026-06-12",
        to: "2026-07-11",
        impressions: 200,
        clicks: 25,
      },
      days: [
        { metricDate: "2026-07-10", impressions: 80, clicks: 10 },
        { metricDate: "2026-07-11", impressions: 120, clicks: 15 },
      ],
    });
    apiMocks.archiveAdminPromotion.mockReset().mockResolvedValue(undefined);
    apiMocks.createAdminPromotion.mockReset();
    apiMocks.updateAdminPromotion.mockReset();
  });

  it("shows the rolling aggregate and an accessible daily privacy-minimal breakdown", async () => {
    const user = userEvent.setup();
    const view = renderPanel();

    expect(await screen.findByText("新生校园指南")).toBeVisible();
    expect(screen.getByText(/200 次曝光 · 25 次点击 · 点击率 12.5%/)).toBeVisible();

    await user.click(screen.getByRole("button", { name: "日趋势" }));
    expect(await screen.findByRole("dialog", { name: /新生校园指南 · 投放趋势/ })).toBeVisible();
    expect(screen.getByRole("table", { name: /按 UTC 日期统计/ })).toBeVisible();
    expect(screen.getByRole("rowheader", { name: "2026-07-11" })).toBeVisible();
    expect(apiMocks.adminPromotionMetrics).toHaveBeenCalledWith("18");
    await expectNoAccessibilityViolations(view.container);
  });
});
