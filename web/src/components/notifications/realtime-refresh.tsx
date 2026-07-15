import { useQueryClient } from "@tanstack/react-query";
import * as React from "react";

import { readAccessToken } from "@/lib/auth-storage";
import { API_BASE_URL } from "@/lib/api/client";
import { accountQueryKeys } from "@/lib/account-query-keys";
import { forumRefreshQueryRoots } from "@/lib/forum-query-keys";
import { consumeSseBuffer } from "@/lib/sse";

const MAX_RECONNECT_DELAY = 30_000;
const FORUM_REFRESH_EVENTS = new Set([
  "sync",
  "reply",
  "mention",
  "quote",
  "vote",
  "watching",
  "flag_auto_hide",
  "mod_action",
  "follow",
]);

export function RealtimeRefresh({
  accountId,
  isAuthenticated,
}: {
  accountId?: string;
  isAuthenticated: boolean;
}) {
  const queryClient = useQueryClient();

  React.useEffect(() => {
    if (!isAuthenticated) return;
    let isStopped = false;
    let reconnectDelay = 1_000;
    let reconnectTimer: number | undefined;
    let controller: AbortController | undefined;

    async function invalidate(eventType: string) {
      const tasks = [
        queryClient.invalidateQueries({ queryKey: accountQueryKeys.notificationCount(accountId) }),
        queryClient.invalidateQueries({ queryKey: accountQueryKeys.notifications(accountId) }),
      ];
      if (eventType === "sync" || ["dm", "dm_request", "dm_request_accepted"].includes(eventType)) {
        tasks.push(
          queryClient.invalidateQueries({ queryKey: accountQueryKeys.directMessageCount(accountId) }),
          queryClient.invalidateQueries({
            queryKey: [...accountQueryKeys.directMessages(accountId), "conversations"],
          }),
        );
      }
      if (FORUM_REFRESH_EVENTS.has(eventType)) {
        tasks.push(...forumRefreshQueryRoots().map((queryKey) =>
          queryClient.invalidateQueries({ queryKey })));
      }
      await Promise.all(tasks);
    }

    function reconnect() {
      if (isStopped) return;
      reconnectTimer = window.setTimeout(() => {
        reconnectTimer = undefined;
        void connect();
      }, reconnectDelay);
      reconnectDelay = Math.min(reconnectDelay * 2, MAX_RECONNECT_DELAY);
    }

    async function connect() {
      const token = readAccessToken();
      if (!token || isStopped) return;
      controller = new AbortController();
      try {
        const response = await fetch(`${API_BASE_URL}/notifications/stream`, {
          headers: {
            Accept: "text/event-stream",
            Authorization: `Bearer ${token}`,
          },
          cache: "no-store",
          signal: controller.signal,
        });
        if (!response.ok || !response.body) throw new Error("notification stream unavailable");
        reconnectDelay = 1_000;
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";
        while (!isStopped) {
          const result = await reader.read();
          if (result.done) break;
          buffer += decoder.decode(result.value, { stream: true });
          const parsed = consumeSseBuffer(buffer);
          buffer = parsed.remainder;
          for (const event of parsed.events) {
            await invalidate(event.event);
          }
        }
        if (!isStopped) reconnect();
      } catch (error) {
        if (!isStopped && !(error instanceof DOMException && error.name === "AbortError")) reconnect();
      }
    }

    void connect();
    return () => {
      isStopped = true;
      controller?.abort();
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
    };
  }, [accountId, isAuthenticated, queryClient]);

  return null;
}
