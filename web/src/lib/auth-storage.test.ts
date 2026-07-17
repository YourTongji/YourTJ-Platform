import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Account } from "@/lib/api/types";

const localDraftMocks = vi.hoisted(() => ({
  allowForAccount: vi.fn(),
  clearForAccount: vi.fn(),
}));

vi.mock("@/lib/local-forum-drafts", () => ({
  allowLocalForumDraftsForAccount: localDraftMocks.allowForAccount,
  clearLocalForumDraftsForAccount: localDraftMocks.clearForAccount,
}));

import {
  clearAuth,
  readAuthContextVersion,
  readOrCreateClientInstallationId,
  writeAuth,
} from "@/lib/auth-storage";

function account(id: string): Account {
  return {
    id,
    handle: `user-${id}`,
    avatarUrl: null,
    role: "user",
    capabilities: [],
    trustLevel: 1,
    hasPassword: false,
    onboardingRequired: false,
    createdAt: 1_700_000_000,
  };
}

describe("client installation identity", () => {
  beforeEach(() => {
    localDraftMocks.allowForAccount.mockReset();
    localDraftMocks.clearForAccount.mockReset().mockResolvedValue(undefined);
  });
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
    expect(localDraftMocks.clearForAccount).toHaveBeenCalledWith("1");
  });

  it("replaces an invalid persisted value with a UUID v4", () => {
    localStorage.setItem("yourtj.clientInstallationId", "not-a-uuid");

    const installationId = readOrCreateClientInstallationId();

    expect(installationId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
    );
    expect(localStorage.getItem("yourtj.clientInstallationId")).toBe(installationId);
  });

  it("clears the previous account recovery copies when the signed-in account changes", () => {
    writeAuth({ accessToken: "first-access", refreshToken: "first-refresh", account: account("1") });
    localDraftMocks.clearForAccount.mockClear();

    writeAuth({ accessToken: "next-access", refreshToken: "next-refresh", account: account("2") });

    expect(localDraftMocks.clearForAccount).toHaveBeenCalledWith("1");
    expect(localDraftMocks.allowForAccount).toHaveBeenCalledWith("2");
  });

  it("keeps a monotonic generation across an A to B to A auth switch", () => {
    const initialVersion = readAuthContextVersion();

    writeAuth({ accessToken: "access-a", refreshToken: "refresh-a", account: account("1") });
    writeAuth({ accessToken: "access-b", refreshToken: "refresh-b", account: account("2") });
    writeAuth({ accessToken: "access-a", refreshToken: "refresh-a", account: account("1") });

    expect(readAuthContextVersion()).toBe(initialVersion + 3);
  });
});
