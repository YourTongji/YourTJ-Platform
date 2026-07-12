import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Info, ShieldAlert } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Announcement } from "@/lib/api/types";

const ANONYMOUS_SEEN_KEY = "yourtj.announcement.seenRevisions";

function announcementKey(announcement: Announcement) {
  return `${announcement.id}:${announcement.revision}`;
}

function readAnonymousSeen() {
  try {
    const parsed = JSON.parse(localStorage.getItem(ANONYMOUS_SEEN_KEY) ?? "[]") as unknown;
    return new Set(Array.isArray(parsed) ? parsed.filter((value): value is string => typeof value === "string") : []);
  } catch {
    return new Set<string>();
  }
}

function rememberAnonymousSeen(announcement: Announcement) {
  const seen = readAnonymousSeen();
  seen.add(announcementKey(announcement));
  try {
    localStorage.setItem(ANONYMOUS_SEEN_KEY, JSON.stringify([...seen].slice(-200)));
  } catch {
    // Browser storage is an enhancement for anonymous visitors; authenticated receipts remain canonical.
  }
}

const severityMeta = {
  info: { label: "平台信息", icon: Info },
  success: { label: "平台进展", icon: CheckCircle2 },
  warning: { label: "重要提醒", icon: AlertTriangle },
  critical: { label: "紧急公告", icon: ShieldAlert },
} as const;

export function AnnouncementModalQueue() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [queue, setQueue] = React.useState<Announcement[]>([]);
  const sourceSignature = React.useRef("");
  const publicAnnouncements = useQuery({
    queryKey: ["announcements", "active", "anonymous"],
    queryFn: api.announcements,
    enabled: !isAuthenticated,
    staleTime: 60_000,
  });
  const unreadAnnouncements = useQuery({
    queryKey: ["announcements", "unread"],
    queryFn: api.unreadAnnouncements,
    enabled: isAuthenticated,
    staleTime: 30_000,
  });
  const source = isAuthenticated ? unreadAnnouncements.data : publicAnnouncements.data;

  React.useEffect(() => {
    if (!source) return;
    const candidates = isAuthenticated
      ? source
      : source.filter((announcement) => !readAnonymousSeen().has(announcementKey(announcement)));
    const signature = `${isAuthenticated ? "account" : "anonymous"}:${candidates.map(announcementKey).join(",")}`;
    if (signature !== sourceSignature.current) {
      sourceSignature.current = signature;
      setQueue(candidates);
    }
  }, [isAuthenticated, source]);

  const current = queue[0];
  React.useEffect(() => {
    if (!current) return;
    if (isAuthenticated) {
      void api
        .recordAnnouncementReceipt(current.id, { revision: current.revision, action: "seen" })
        .catch(() => toast.error("公告已显示，但已读状态暂未同步"));
    } else {
      rememberAnonymousSeen(current);
    }
  }, [current, isAuthenticated]);

  const completeCurrent = React.useCallback(() => {
    setQueue((items) => items.slice(1));
  }, []);

  const recordAndComplete = React.useCallback(
    async (action: "dismiss" | "acknowledge") => {
      if (!current) return;
      if (!isAuthenticated) {
        completeCurrent();
        return;
      }
      try {
        await api.recordAnnouncementReceipt(current.id, {
          revision: current.revision,
          action,
        });
        completeCurrent();
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ["announcements", "active"] }),
          queryClient.invalidateQueries({ queryKey: ["announcements", "unread"] }),
        ]);
      } catch (error) {
        toast.error(error instanceof Error ? error.message : "公告状态同步失败，请重试");
      }
    },
    [completeCurrent, current, isAuthenticated, queryClient],
  );

  if (!current) return null;
  const meta = severityMeta[current.severity];
  const SeverityIcon = meta.icon;

  return (
    <Dialog open onOpenChange={(open) => !open && void recordAndComplete("dismiss")}>
      <DialogContent aria-describedby="announcement-modal-description">
        <DialogHeader>
          <Badge variant={current.severity === "critical" ? "destructive" : "secondary"} className="w-fit">
            <SeverityIcon className="size-3.5" aria-hidden="true" />
            {meta.label}
          </Badge>
          <DialogTitle>{current.title}</DialogTitle>
          <DialogDescription id="announcement-modal-description">
            公告版本 {current.revision}
            {current.requiresAck ? " · 需要明确确认" : " · 本版本只展示一次"}
          </DialogDescription>
        </DialogHeader>
        {current.body ? (
          <div className="max-h-[50vh] overflow-y-auto whitespace-pre-wrap text-sm leading-7 text-foreground">
            {current.body}
          </div>
        ) : null}
        <DialogFooter>
          {current.requiresAck ? (
            <Button type="button" variant="outline" onClick={() => void recordAndComplete("dismiss")}>
              稍后处理
            </Button>
          ) : null}
          <Button
            type="button"
            onClick={() => void recordAndComplete(current.requiresAck ? "acknowledge" : "dismiss")}
          >
            {current.requiresAck ? "我已知晓" : "知道了"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
