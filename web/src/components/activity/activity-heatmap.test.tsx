import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import type { ActivityCalendar } from "@/lib/api/types";
import { TooltipProvider } from "@/components/ui/tooltip";

import { ActivityHeatmap } from "./activity-heatmap";

const calendar = {
  from: "2026-02-23",
  to: "2026-07-12",
  timezone: "Asia/Shanghai",
  policyVersion: 4,
  trustPolicyVersion: 7,
  likeDailyCap: 20,
  weights: { thread: 10, comment: 3, like: 1, checkIn: 2 },
  days: [
    {
      date: "2026-02-23",
      threads: 1,
      comments: 2,
      likes: 3,
      checkIns: 1,
      score: 21,
    },
    {
      date: "2026-02-24",
      threads: 0,
      comments: 1,
      likes: 0,
      checkIns: 1,
      score: 5,
    },
  ],
} as ActivityCalendar;

function renderHeatmap(props: Partial<React.ComponentProps<typeof ActivityHeatmap>> = {}) {
  render(
    <MemoryRouter>
      <TooltipProvider>
        <ActivityHeatmap
          isAuthenticated
          calendar={calendar}
          isLoading={false}
          onRetry={vi.fn()}
          {...props}
        />
      </TooltipProvider>
    </MemoryRouter>,
  );
}

describe("ActivityHeatmap", () => {
  it("explains the configured score formula and exposes check-ins per day", () => {
    renderHeatmap();

    expect(screen.getByText(/发帖 ×10 · 评论 ×3 · 点赞 ×1/)).toHaveTextContent(
      "每日最多 20 分",
    );
    expect(screen.getByText(/发帖 ×10 · 评论 ×3 · 点赞 ×1/)).toHaveTextContent("签到 ×2");
    expect(screen.getByRole("gridcell", { name: /活跃度 21 分/ })).toHaveAccessibleName(
      /签到 1/,
    );
  });

  it("supports keyboard movement between populated dates", () => {
    renderHeatmap();
    const first = screen.getByRole("gridcell", { name: /活跃度 21 分/ });
    const second = screen.getByRole("gridcell", { name: /活跃度 5 分/ });

    first.focus();
    fireEvent.keyDown(first, { key: "ArrowDown" });

    expect(second).toHaveFocus();
  });

  it("does not fabricate an activity calendar for visitors", () => {
    renderHeatmap({ isAuthenticated: false, calendar: undefined });

    expect(screen.getByRole("link", { name: "登录查看" })).toHaveAttribute("href", "/login");
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });
});
