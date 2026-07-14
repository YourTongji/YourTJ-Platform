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
});
