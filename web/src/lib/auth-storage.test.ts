import { afterEach, describe, expect, it } from "vitest";

import {
  clearAuth,
  readOrCreateClientInstallationId,
} from "@/lib/auth-storage";

describe("client installation identity", () => {
  afterEach(() => localStorage.clear());

  it("survives normal sign-out without retaining account credentials", () => {
    const installationId = readOrCreateClientInstallationId();
    localStorage.setItem("yourtj.accessToken", "access-token");
    localStorage.setItem("yourtj.refreshToken", "refresh-token");
    localStorage.setItem("yourtj.account", JSON.stringify({ id: "1", handle: "alice" }));

    clearAuth();

    expect(readOrCreateClientInstallationId()).toBe(installationId);
    expect(localStorage.getItem("yourtj.accessToken")).toBeNull();
    expect(localStorage.getItem("yourtj.refreshToken")).toBeNull();
    expect(localStorage.getItem("yourtj.account")).toBeNull();
  });

  it("replaces an invalid persisted value with a UUID v4", () => {
    localStorage.setItem("yourtj.clientInstallationId", "not-a-uuid");

    const installationId = readOrCreateClientInstallationId();

    expect(installationId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
    );
    expect(localStorage.getItem("yourtj.clientInstallationId")).toBe(installationId);
  });
});
