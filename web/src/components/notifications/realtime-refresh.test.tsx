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
        <RealtimeRefresh isAuthenticated />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/notifications/stream"),
      expect.objectContaining({
        headers: expect.objectContaining({ Authorization: "Bearer access-token" }),
      }),
    ));
    await waitFor(() => expect(invalidate).toHaveBeenCalledWith({ queryKey: ["dm-unread-count"] }));
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["notification-count"] });
  });
});
