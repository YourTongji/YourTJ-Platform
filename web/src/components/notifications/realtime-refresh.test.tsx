import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { RealtimeRefresh } from "./realtime-refresh";

describe("RealtimeRefresh", () => {
  afterEach(() => {
    localStorage.clear();
    vi.unstubAllGlobals();
  });

  it("uses an authorization header and treats SSE as a query refresh hint", async () => {
    localStorage.setItem("yourtj.accessToken", "access-token");
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      "event: dm_request\ndata: {}\n\n",
      { status: 200, headers: { "Content-Type": "text/event-stream" } },
    ));
    vi.stubGlobal("fetch", fetchMock);
    const queryClient = new QueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries").mockResolvedValue(undefined);

    render(
      <QueryClientProvider client={queryClient}>
        <RealtimeRefresh accountId="account-1" isAuthenticated />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/notifications/stream"),
      expect.objectContaining({
        headers: expect.objectContaining({ Authorization: "Bearer access-token" }),
      }),
    ));
    await waitFor(() => expect(invalidate).toHaveBeenCalledWith({
      queryKey: ["dm-unread-count", "account-1"],
    }));
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ["notification-count", "account-1"],
    });
  });

  it("refreshes private messages after reconnect sync because intermediate hints may be lost", async () => {
    localStorage.setItem("yourtj.accessToken", "access-token");
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(
      "event: sync\ndata: {}\n\n",
      { status: 200, headers: { "Content-Type": "text/event-stream" } },
    )));
    const queryClient = new QueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries").mockResolvedValue(undefined);

    render(
      <QueryClientProvider client={queryClient}>
        <RealtimeRefresh accountId="account-2" isAuthenticated />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(invalidate).toHaveBeenCalledWith({
      queryKey: ["dm", "account-2", "conversations"],
    }));
  });
});
