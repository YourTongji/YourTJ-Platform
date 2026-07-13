import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import type { Account, CheckInStatus } from "@/lib/api/types";

import { DailyCheckInButton } from "./daily-check-in-button";

const account = { id: "17", handle: "alice" } as Account;
const unchecked = {
  checkedIn: false,
  newlyCheckedIn: false,
  timezone: "Asia/Shanghai",
  checkedInAt: null,
  currentStreak: 4,
  totalDays: 19,
  date: "2026-07-12",
  nextResetAt: 1_784_044_800,
} as CheckInStatus;

function renderButton(overrides: Partial<React.ComponentProps<typeof DailyCheckInButton>> = {}) {
  const props: React.ComponentProps<typeof DailyCheckInButton> = {
    account,
    status: unchecked,
    isLoading: false,
    isPending: false,
    onCheckIn: vi.fn(),
    onRetry: vi.fn(),
    ...overrides,
  };
  render(
    <MemoryRouter>
      <DailyCheckInButton {...props} />
    </MemoryRouter>,
  );
  return props;
}

describe("DailyCheckInButton", () => {
  it("invokes the idempotent check-in action and exposes the accumulated days", async () => {
    const user = userEvent.setup();
    const props = renderButton();

    await user.click(screen.getByRole("button", { name: "每日签到 · 累计 19 天" }));

    expect(props.onCheckIn).toHaveBeenCalledOnce();
  });

  it("disables a completed day and shows the current streak", () => {
    renderButton({
      status: { ...unchecked, checkedIn: true, currentStreak: 5 },
    });

    expect(screen.getByRole("button", { name: "今日已签到 · 连续 5 天" })).toBeDisabled();
  });

  it("offers a retry when the status request fails", async () => {
    const user = userEvent.setup();
    const props = renderButton({ error: new Error("offline"), status: undefined });

    await user.click(screen.getByRole("button", { name: "签到状态加载失败，重试" }));

    expect(props.onRetry).toHaveBeenCalledOnce();
    expect(props.onCheckIn).not.toHaveBeenCalled();
  });

  it("keeps the action reachable for signed-out visitors", () => {
    renderButton({ account: null, status: undefined });

    expect(screen.getByRole("link", { name: "登录后每日签到" })).toHaveAttribute(
      "href",
      "/login",
    );
  });
});
