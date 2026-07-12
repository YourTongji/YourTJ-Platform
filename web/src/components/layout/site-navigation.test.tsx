import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { SiteSidebar } from "./site-navigation";

const apiMocks = vi.hoisted(() => ({ promotions: vi.fn(), recordPromotionEvent: vi.fn() }));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ account: null, isAuthenticated: false }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    mediaUrl: vi.fn(),
    promotions: apiMocks.promotions,
    recordPromotionEvent: apiMocks.recordPromotionEvent,
  },
}));

function renderSidebar() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <SiteSidebar />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("SiteSidebar promotions", () => {
  beforeEach(() => {
    vi.stubGlobal("IntersectionObserver", class {
      private readonly callback: IntersectionObserverCallback;

      constructor(callback: IntersectionObserverCallback) {
        this.callback = callback;
      }

      observe(target: Element) {
        this.callback([{
          target,
          isIntersecting: true,
          intersectionRatio: 1,
        } as IntersectionObserverEntry], this as unknown as IntersectionObserver);
      }

      disconnect() {}
    });
    apiMocks.recordPromotionEvent.mockReset().mockResolvedValue(undefined);
    apiMocks.promotions.mockReset().mockResolvedValue([
      {
        id: "5",
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
        version: 1,
        startsAt: null,
        endsAt: null,
        archivedAt: null,
        createdAt: 1_700_000_000,
        updatedAt: 1_700_000_000,
        trackingToken: "primary-presentation-token",
        metrics: null,
      },
      {
        id: "6",
        placement: "home-left-primary",
        title: "同位置低优先级内容",
        body: null,
        ctaLabel: null,
        targetUrl: "/forum",
        assetId: null,
        status: "published",
        effectiveState: "active",
        priority: 1,
        audience: "all",
        version: 1,
        startsAt: null,
        endsAt: null,
        archivedAt: null,
        createdAt: 1_700_000_000,
        updatedAt: 1_700_000_000,
        trackingToken: "secondary-presentation-token",
        metrics: null,
      },
    ]);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("renders API-backed first-party promotion content without placeholder advertising", async () => {
    const user = userEvent.setup();
    const view = renderSidebar();
    const promotion = await screen.findByRole("link", { name: /新生校园指南/ });
    expect(promotion).toHaveAttribute("href", "/forum/threads/8");
    expect(screen.queryByText("ADVERTISEMENT")).not.toBeInTheDocument();
    expect(screen.queryByText("同位置低优先级内容")).not.toBeInTheDocument();
    expect(screen.getByText("社区推广")).toBeVisible();
    await waitFor(() => {
      expect(apiMocks.recordPromotionEvent).toHaveBeenCalledWith(
        "5",
        "impression",
        "primary-presentation-token",
      );
    });
    await user.click(promotion);
    expect(apiMocks.recordPromotionEvent).toHaveBeenCalledWith(
      "5",
      "click",
      "primary-presentation-token",
    );
    await expectNoAccessibilityViolations(view.container);
  });
});
