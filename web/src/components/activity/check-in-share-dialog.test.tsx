import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CheckInStatus, TrustProgress } from "@/lib/api/types";

import {
  buildCheckInShareData,
  CheckInShareDialog,
  createCheckInShareSvg,
  createCheckInShareText,
} from "./check-in-share-dialog";

const status = {
  checkedIn: true,
  newlyCheckedIn: true,
  timezone: "Asia/Shanghai",
  checkedInAt: 1_784_044_000,
  currentStreak: 5,
  totalDays: 20,
  date: "2026-07-14",
  nextResetAt: 1_784_044_800,
} as CheckInStatus;

const trustProgress = {
  trustLevel: 3,
  teaName: "红茶",
  progressPercent: 72,
} as TrustProgress;

describe("CheckInShareDialog", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("exports only the PII allowlist even when source objects contain forbidden fields", () => {
    const hostileStatus = {
      ...status,
      email: "alice@tongji.edu.cn",
      accountId: "account-secret",
      deviceId: "device-secret",
      deliveryUrl: "https://cdn.example/signed-secret?token=secret",
    } as CheckInStatus;
    const hostileProgress = {
      ...trustProgress,
      accountId: "account-secret",
      avatarUrl: "https://cdn.example/avatar?signature=secret",
    } as TrustProgress;

    const data = buildCheckInShareData(hostileStatus, hostileProgress);
    const exported = [JSON.stringify(data), createCheckInShareText(data), createCheckInShareSvg(data)].join("\n");

    expect(data).toEqual({
      date: "2026-07-14",
      currentStreak: 5,
      totalDays: 20,
      trustLevel: 3,
      teaName: "红茶",
      progressPercent: 72,
    });
    expect(exported).not.toMatch(/alice@|account-secret|device-secret|cdn\.example|token=|signature=/);
  });

  it("copies the allowlisted share text", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });

    render(
      <CheckInShareDialog
        open
        onOpenChange={vi.fn()}
        status={status}
        trustProgress={trustProgress}
      />,
    );
    await user.click(screen.getByRole("button", { name: "复制文案" }));

    expect(writeText).toHaveBeenCalledWith(
      "我在 YourTJ 完成了 2026-07-14 每日签到！\n连续 5 天 · 累计 20 天\n当前成长：Lv.3 · 红茶",
    );
  });
});
