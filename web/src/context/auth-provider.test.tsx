import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AuthProvider, useAuth } from "./auth-provider";
import { clearAuth } from "@/lib/auth-storage";

const apiMocks = vi.hoisted(() => ({
  me: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({
  api: { me: apiMocks.me },
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn() },
}));

function CurrentAccount() {
  const { account, isLoading } = useAuth();
  return <span>{isLoading ? "loading" : account?.id ?? "signed-out"}</span>;
}

describe("AuthProvider", () => {
  afterEach(() => {
    localStorage.clear();
  });

  it("clears private query data before adopting a different authenticated account", async () => {
    localStorage.setItem("yourtj.accessToken", "account-a-access");
    localStorage.setItem("yourtj.refreshToken", "account-a-refresh");
    localStorage.setItem("yourtj.account", JSON.stringify({ id: "account-a", handle: "alice" }));
    apiMocks.me.mockReset().mockResolvedValue({ id: "account-b", handle: "bob" });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    queryClient.setQueryData(["notifications", "account-a"], { items: [{ id: "private-a" }] });

    render(
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <CurrentAccount />
        </AuthProvider>
      </QueryClientProvider>,
    );

    expect(await screen.findByText("account-b")).toBeVisible();
    await waitFor(() => {
      expect(queryClient.getQueryData(["notifications", "account-a"])).toBeUndefined();
    });
  });

  it("clears the active principal and private cache when refresh loses credentials locally", async () => {
    localStorage.setItem("yourtj.accessToken", "account-a-access");
    localStorage.setItem("yourtj.refreshToken", "account-a-refresh");
    localStorage.setItem("yourtj.account", JSON.stringify({ id: "account-a", handle: "alice" }));
    apiMocks.me.mockReset().mockResolvedValue({ id: "account-a", handle: "alice" });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    queryClient.setQueryData(["notifications", "account-a"], { items: [{ id: "private-a" }] });

    render(
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <CurrentAccount />
        </AuthProvider>
      </QueryClientProvider>,
    );

    expect(await screen.findByText("account-a")).toBeVisible();
    act(() => clearAuth());
    expect(await screen.findByText("signed-out")).toBeVisible();
    await waitFor(() => {
      expect(queryClient.getQueryData(["notifications", "account-a"])).toBeUndefined();
    });
  });
});
