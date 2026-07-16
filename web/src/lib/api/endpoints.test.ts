import { afterEach, describe, expect, it, vi } from "vitest";

import { api } from "@/lib/api/endpoints";

describe("authentication endpoint metadata", () => {
  afterEach(() => {
    localStorage.clear();
    vi.unstubAllGlobals();
  });

  it("sends one stable first-party installation identifier on full login", async () => {
    const requestBodies: Array<Record<string, unknown>> = [];
    const fetchMock = vi.fn().mockImplementation((_url: URL, init?: RequestInit) => {
      requestBodies.push(JSON.parse(String(init?.body)) as Record<string, unknown>);
      return Promise.resolve(new Response(JSON.stringify({
        accessToken: "access",
        refreshToken: "refresh",
        account: { id: "1", handle: "alice" },
      }), { headers: { "Content-Type": "application/json" } }));
    });
    vi.stubGlobal("fetch", fetchMock);

    await api.passwordLogin({ email: "alice@tongji.edu.cn", password: "password" });
    await api.verifyEmail({
      email: "alice@tongji.edu.cn",
      code: "123456",
      purpose: "login",
    });

    expect(requestBodies[0].clientInstallationId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
    );
    expect(requestBodies[1].clientInstallationId).toBe(requestBodies[0].clientInstallationId);
  });

  it("keeps a signing intent id out of the outcome request URL", async () => {
    const intentId = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      intentId,
      status: "pending",
      expiresAt: 1_800_000_300,
    }), { headers: { "Content-Type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);

    await api.creditSigningIntentOutcome(intentId, "fixed-access-token");

    const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit];
    expect(url.pathname).toBe("/api/v2/credit/signing-intent-outcome");
    expect(url.href).not.toContain(intentId);
    expect(init.method).toBe("POST");
    expect(JSON.parse(String(init.body))).toEqual({ intentId });
    expect(new Headers(init.headers).get("Authorization")).toBe("Bearer fixed-access-token");
  });

  it("uses the explicitly captured token for wallet key enrollment", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await api.bindWallet("account-a", "public-key-a", "verified-access-a");

    const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit];
    expect(url.pathname).toBe("/api/v2/wallet/bind");
    expect(new Headers(init.headers).get("Authorization")).toBe("Bearer verified-access-a");
    expect(JSON.parse(String(init.body))).toEqual({
      accountId: "account-a",
      publicKey: "public-key-a",
    });
  });
});
