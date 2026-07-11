import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Clock3, Download, FileJson, Power, ShieldAlert, Trash2 } from "lucide-react";
import * as React from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";

import { RecentAuthDialog } from "@/components/auth/recent-auth-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/context/auth-provider";
import { storeRecoveryCredential } from "@/lib/account-recovery";
import { ApiError } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import type { AccountLifecycleMutationInput, DataExportJob } from "@/lib/api/types";
import { randomUuid } from "@/lib/random";

type SecureAction = "export" | "download" | "deactivate" | "delete";
type ClosureAction = Exclude<SecureAction, "export" | "download">;

const EXPORT_SESSION_KEY = "yourtj.latestDataExport";

const exportStatusLabel: Record<DataExportJob["status"], string> = {
  queued: "等待处理",
  running: "正在整理",
  ready: "可下载",
  failed: "处理失败",
  expired: "已过期",
};

function readExportId() {
  return sessionStorage.getItem(EXPORT_SESSION_KEY);
}

function formatTimestamp(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" })
    .format(new Date(timestamp * 1_000));
}

function ClosureConfirmation({
  action,
  open,
  onOpenChange,
  onContinue,
}: {
  action: ClosureAction;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onContinue: () => void;
}) {
  const isDelete = action === "delete";
  const phrase = isDelete ? "删除账号" : "停用账号";
  const [confirmation, setConfirmation] = React.useState("");

  React.useEffect(() => {
    if (!open) setConfirmation("");
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <ShieldAlert className="size-5" aria-hidden="true" />
            {isDelete ? "申请删除账号" : "停用账号"}
          </DialogTitle>
          <DialogDescription>
            {isDelete
              ? "提交后立即退出所有设备、停止新的社区互动，并进入 30 天恢复期；到期后后台任务会清除或匿名化可清除的账号数据。"
              : "提交后立即退出所有设备并隐藏账号。你可以随时通过校园邮箱或密码恢复。"}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="rounded-lg border border-destructive/20 bg-destructive/[0.04] p-4 text-sm leading-6 text-muted-foreground">
            {isDelete ? (
              <ul className="list-disc space-y-1 pl-5">
                <li>公开内容会按社区完整性与治理规则保留或匿名化，不承诺从引用、备份或审计记录中即时消失。</li>
                <li>积分账本为防篡改记录，必须保留；清除后只保留不可反查的 tombstone。</li>
                <li>30 天内恢复会取消尚未执行的清除任务，但不会恢复旧会话。</li>
              </ul>
            ) : "停用不是删除：内容和账号数据仍会保留，只是账号不再公开或参与新的互动。"}
          </div>
          <div className="space-y-2">
            <Label htmlFor={`confirm-${action}`}>输入“{phrase}”以继续</Label>
            <Input
              id={`confirm-${action}`}
              value={confirmation}
              onChange={(event) => setConfirmation(event.target.value)}
              autoComplete="off"
            />
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button type="button" variant="destructive" disabled={confirmation !== phrase} onClick={onContinue}>
            继续安全验证
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function AccountDataSettings() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { clearSession } = useAuth();
  const [exportId, setExportId] = React.useState<string | null>(() => readExportId());
  const [confirmationAction, setConfirmationAction] = React.useState<ClosureAction | null>(null);
  const [secureAction, setSecureAction] = React.useState<SecureAction | null>(null);
  const [recentAuthOpen, setRecentAuthOpen] = React.useState(false);

  const lifecycle = useQuery({ queryKey: ["account-lifecycle"], queryFn: api.accountLifecycle });
  const recentExports = useQuery({
    queryKey: ["account-data-exports"],
    queryFn: api.dataExports,
    enabled: exportId === null,
  });
  const exportJob = useQuery({
    queryKey: ["account-data-export", exportId],
    queryFn: () => api.dataExport(exportId ?? ""),
    enabled: Boolean(exportId),
    retry: false,
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      return status === "queued" || status === "running" ? 2_000 : false;
    },
  });

  React.useEffect(() => {
    if (!(exportJob.error instanceof ApiError) || exportJob.error.status !== 404 || !exportId) return;
    sessionStorage.removeItem(EXPORT_SESSION_KEY);
    setExportId(null);
  }, [exportId, exportJob.error]);

  React.useEffect(() => {
    if (exportId || !recentExports.data?.[0]) return;
    const latestId = recentExports.data[0].id;
    sessionStorage.setItem(EXPORT_SESSION_KEY, latestId);
    setExportId(latestId);
  }, [exportId, recentExports.data]);

  const createExport = useMutation({
    mutationFn: () => api.createDataExport(randomUuid()),
    onSuccess: (job) => {
      sessionStorage.setItem(EXPORT_SESSION_KEY, job.id);
      setExportId(job.id);
      queryClient.setQueryData(["account-data-export", job.id], job);
      toast.success("数据导出已开始，可离开此页面后再回来查看");
    },
  });

  const downloadExport = useMutation({
    mutationFn: async () => {
      if (!exportId) throw new Error("没有可下载的导出");
      const grant = await api.createDataExportDownloadGrant(exportId);
      return api.downloadDataExport(exportId, grant.token);
    },
    onSuccess: (artifact) => {
      const blob = new Blob([JSON.stringify(artifact, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `yourtj-account-export-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      toast.success("数据导出已下载；一次性下载凭证现已失效");
    },
  });

  const closeAccount = useMutation({
    mutationFn: (action: ClosureAction) => {
      const input: AccountLifecycleMutationInput = {
        confirmation: action === "delete" ? "DELETE" : "DEACTIVATE",
      };
      return action === "delete"
        ? api.deleteAccount(input, randomUuid())
        : api.deactivateAccount(input, randomUuid());
    },
    onSuccess: (result, action) => {
      storeRecoveryCredential(result.recovery);
      clearSession();
      queryClient.clear();
      toast.success(action === "delete" ? "账号已进入 30 天删除恢复期" : "账号已停用");
      navigate("/recover-account", { replace: true });
    },
  });

  function requestSecureAction(action: SecureAction) {
    setSecureAction(action);
    setRecentAuthOpen(true);
  }

  function executeSecureAction() {
    const action = secureAction;
    setSecureAction(null);
    if (action === "export") createExport.mutate();
    else if (action === "download") downloadExport.mutate();
    else if (action) closeAccount.mutate(action);
  }

  const job = exportJob.data;
  const error = createExport.error ?? downloadExport.error ?? closeAccount.error;

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>账号数据与生命周期</CardTitle>
          <CardDescription>导出自己的数据，或控制账号停用与删除。所有操作都要求当前设备重新验证。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <section className="space-y-3" aria-labelledby="account-export-title">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <h3 id="account-export-title" className="flex items-center gap-2 font-medium"><FileJson className="size-4 text-primary" aria-hidden="true" />下载你的数据</h3>
                <p className="mt-1 max-w-xl text-sm leading-6 text-muted-foreground">生成机器可读 JSON，包含身份、自己发布的内容、社交关系、积分记录及媒体元数据；不会包含举报人、工作人员、内部证据或收到的私信正文。</p>
              </div>
              {!job || job.status === "failed" || job.status === "expired" ? (
                <Button type="button" variant="outline" onClick={() => requestSecureAction("export")} disabled={createExport.isPending || recentExports.isLoading}>
                  {createExport.isPending ? "正在创建…" : recentExports.isLoading ? "正在检查…" : "创建导出"}
                </Button>
              ) : null}
            </div>
            {job ? (
              <div className="flex flex-col gap-3 rounded-lg border bg-muted/20 p-4 sm:flex-row sm:items-center sm:justify-between" aria-live="polite">
                <div>
                  <p className="font-medium">{exportStatusLabel[job.status]}</p>
                  <p className="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground"><Clock3 className="size-3.5" aria-hidden="true" />导出在 {formatTimestamp(job.expiresAt)} 后自动过期并清除</p>
                  {job.errorCode ? <p className="mt-1 text-xs text-destructive">错误代码：{job.errorCode}</p> : null}
                </div>
                {job.status === "ready" ? (
                  <Button type="button" onClick={() => requestSecureAction("download")} disabled={downloadExport.isPending}>
                    <Download className="size-4" aria-hidden="true" />{downloadExport.isPending ? "正在下载…" : "一次性下载"}
                  </Button>
                ) : null}
              </div>
            ) : null}
          </section>

          <div className="h-px bg-border" />

          <section className="space-y-4" aria-labelledby="account-close-title">
            <div>
              <h3 id="account-close-title" className="font-medium">关闭账号</h3>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">当前状态：{lifecycle.data?.state === "active" ? "正常" : "读取中"}。停用适合暂时离开；删除申请会启动 30 天恢复期和后续清除任务。</p>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-lg border p-4">
                <Power className="size-5 text-muted-foreground" aria-hidden="true" />
                <p className="mt-3 font-medium">停用账号</p>
                <p className="mt-1 text-sm leading-6 text-muted-foreground">立即退出并隐藏账号，不启动数据清除；之后可以随时恢复。</p>
                <Button className="mt-4" type="button" variant="outline" onClick={() => setConfirmationAction("deactivate")}>停用账号</Button>
              </div>
              <div className="rounded-lg border border-destructive/30 p-4">
                <Trash2 className="size-5 text-destructive" aria-hidden="true" />
                <p className="mt-3 font-medium">删除账号</p>
                <p className="mt-1 text-sm leading-6 text-muted-foreground">立即关闭账号，并在 30 天恢复期结束后清除或匿名化可清除数据。</p>
                <Button className="mt-4" type="button" variant="destructive" onClick={() => setConfirmationAction("delete")}>申请删除</Button>
              </div>
            </div>
          </section>

          {error ? <p className="text-sm text-destructive" role="alert">{error instanceof Error ? error.message : "操作失败，请重试"}</p> : null}
        </CardContent>
      </Card>

      {confirmationAction ? (
        <ClosureConfirmation
          action={confirmationAction}
          open
          onOpenChange={(open) => { if (!open) setConfirmationAction(null); }}
          onContinue={() => {
            const action = confirmationAction;
            setConfirmationAction(null);
            requestSecureAction(action);
          }}
        />
      ) : null}
      <RecentAuthDialog
        open={recentAuthOpen}
        onOpenChange={setRecentAuthOpen}
        onVerified={executeSecureAction}
        description="导出账号数据、停用或申请删除前，需要当前设备在最近 10 分钟内重新验证。"
      />
    </>
  );
}
