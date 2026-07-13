import { describe, expect, it } from "vitest";

import { shouldMountAnnouncementQueue } from "./app-layout";

describe("announcement queue boot gating", () => {
  it("waits for auth resolution before choosing anonymous or account receipts", () => {
    expect(shouldMountAnnouncementQueue(true, false, false)).toBe(false);
    expect(shouldMountAnnouncementQueue(false, true, false)).toBe(true);
  });

  it("keeps focused onboarding free from global announcement interruptions", () => {
    expect(shouldMountAnnouncementQueue(false, true, true)).toBe(false);
    expect(shouldMountAnnouncementQueue(false, false, false)).toBe(true);
  });
});
