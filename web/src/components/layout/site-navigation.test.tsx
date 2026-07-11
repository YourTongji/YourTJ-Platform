import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { SiteSidebar } from "./site-navigation";

const apiMocks = vi.hoisted(() => ({ promotions: vi.fn() }));

vi.mock("@/context/auth-provider", () => ({
  useAuth: () => ({ account: null, isAuthenticated: false }),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: {
    mediaUrl: vi.fn(),
    promotions: apiMocks.promotions,
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
      },
    ]);
  });

  it("renders API-backed first-party promotion content without placeholder advertising", async () => {
    const view = renderSidebar();
    const promotion = await screen.findByRole("link", { name: /新生校园指南/ });
    expect(promotion).toHaveAttribute("href", "/forum/threads/8");
    expect(screen.queryByText("ADVERTISEMENT")).not.toBeInTheDocument();
    expect(screen.queryByText("同位置低优先级内容")).not.toBeInTheDocument();
    await expectNoAccessibilityViolations(view.container);
  });
});
