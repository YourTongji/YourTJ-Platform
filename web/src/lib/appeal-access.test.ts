import { beforeEach, describe, expect, it } from "vitest";

import {
  clearAppealAccess,
  readAppealAccess,
  writeAppealAccess,
} from "@/lib/appeal-access";

describe("appeal access cache partition", () => {
  beforeEach(() => sessionStorage.clear());

  it("assigns a new opaque cache partition to every restricted credential", () => {
    const expiresAt = Math.floor(Date.now() / 1_000) + 3_600;
    const first = writeAppealAccess({ accessToken: "first-token", expiresAt });
    const second = writeAppealAccess({ accessToken: "second-token", expiresAt });

    expect(first.cachePartition).not.toBe(second.cachePartition);
    expect(readAppealAccess()).toEqual(second);
  });

  it("upgrades a valid legacy credential without deriving a key from its token", () => {
    const legacy = {
      accessToken: "private-token-that-must-not-enter-a-query-key",
      expiresAt: Math.floor(Date.now() / 1_000) + 3_600,
    };
    sessionStorage.setItem("yourtj.appealAccess", JSON.stringify(legacy));

    const upgraded = readAppealAccess();
    expect(upgraded?.accessToken).toBe(legacy.accessToken);
    expect(upgraded?.cachePartition).not.toContain(legacy.accessToken);
    expect(JSON.parse(sessionStorage.getItem("yourtj.appealAccess") ?? "{}")).toEqual(upgraded);

    clearAppealAccess();
    expect(readAppealAccess()).toBeNull();
  });
});
