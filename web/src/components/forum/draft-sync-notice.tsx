import { AlertTriangle, Cloud, LoaderCircle } from "lucide-react";

import { Button } from "@/components/ui/button";
import type {
  DraftSyncStatus,
  LocalDraftBackupStatus,
} from "@/components/forum/use-forum-draft";
import { formatUnixTime } from "@/lib/format";

interface DraftSyncNoticeProps {
  status: DraftSyncStatus;
  localBackupStatus: LocalDraftBackupStatus;
  savedAt: number | null;
  onRestoreRemote: () => void;
  onKeepLocal: () => void;
  onRetry: () => void;
}

export function DraftSyncNotice({
  status,
  localBackupStatus,
  savedAt,
  onRestoreRemote,
  onKeepLocal,
  onRetry,
}: DraftSyncNoticeProps) {
  if (status === "disabled") return null;
  if (status === "conflict") {
    return (
      <div role="alert" className="flex flex-col gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm sm:flex-row sm:items-center sm:justify-between">
        <span className="flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 shrink-0 text-amber-600" />
          另一设备更新了这份草稿。请选择要继续的版本。
        </span>
        <span className="flex flex-wrap gap-2">
          <Button type="button" size="sm" variant="outline" onClick={onRestoreRemote}>载入云端</Button>
          <Button type="button" size="sm" onClick={onKeepLocal}>保留当前</Button>
        </span>
      </div>
    );
  }
  if (status === "error") {
    return (
      <div role="alert" className="flex items-center justify-between gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm">
        <span>
          {localBackupStatus === "saved"
            ? "云端草稿暂时无法同步，已从本机恢复副本保留当前输入。"
            : "云端草稿暂时无法同步，当前输入仍保留在本页。"}
        </span>
        <Button type="button" size="sm" variant="outline" onClick={onRetry}>重试</Button>
      </div>
    );
  }

  const isBusy = status === "loading" || status === "saving";
  const label = status === "loading"
    ? "正在检查云端草稿"
    : status === "saving"
      ? "正在保存草稿"
      : status === "saved" && savedAt
        ? `草稿已保存 · ${formatUnixTime(savedAt)}`
        : "输入后将自动保存到云端";
  const localLabel = localBackupStatus === "saved"
    ? "本机恢复副本已更新"
    : localBackupStatus === "saving"
      ? "正在更新本机副本"
      : localBackupStatus === "error" || localBackupStatus === "unavailable"
        ? "本机恢复副本不可用"
        : null;
  return (
    <p role="status" aria-live="polite" className="flex items-center gap-2 text-xs text-muted-foreground">
      {isBusy ? (
        <LoaderCircle className="h-3.5 w-3.5 animate-spin motion-reduce:animate-none" />
      ) : (
        <Cloud className="h-3.5 w-3.5" />
      )}
      {label}
      {localLabel ? ` · ${localLabel}` : null}
    </p>
  );
}
