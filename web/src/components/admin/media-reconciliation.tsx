import { RefreshCw, ScanSearch, ShieldAlert } from "lucide-react";

import { PaginationControls } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type {
  MediaReconciliationIssueCode,
  MediaReconciliationReport,
} from "@/lib/api/types";
import { formatNumber } from "@/lib/format";

const ISSUE_LABELS: Record<MediaReconciliationIssueCode, string> = {
  publication_missing: "缺少资产发布记录",
  published_variant_set_incomplete: "已发布的安全版本集合不完整",
  published_upload_not_clean: "已发布资产的审核状态不是 clean",
  hidden_asset_has_published_variant: "已隐藏资产仍有 published 变体",
  processing_lease_stale: "安全版本处理租约已过期",
  processing_without_active_job: "processing 状态没有活动处理任务",
  failed_publication_job_mismatch: "failed 发布状态与死信任务不一致",
  deletion_lease_stale: "删除任务租约已过期",
  cleanup_lease_stale: "清理步骤租约已过期",
  hidden_without_active_deletion_job: "已隐藏资产没有活动删除任务",
  cleanup_plan_incomplete: "CDN、Delivery 与 Ingest 清理计划不完整",
  deletion_completion_pending: "清理步骤已完成但删除任务尚未收敛",
  deletion_dead_letter: "删除任务已进入死信",
  processing_dead_letter: "安全版本处理任务已进入死信",
};

export function MediaReconciliation({
  report,
  isLoading,
  isFetching,
  hasError,
  error,
  requiresRecentAuth,
  hasPrevious,
  onRefresh,
  onPrevious,
  onNext,
  onRequestRecentAuth,
}: {
  report?: MediaReconciliationReport;
  isLoading: boolean;
  isFetching: boolean;
  hasError: boolean;
  error: unknown;
  requiresRecentAuth: boolean;
  hasPrevious: boolean;
  onRefresh: () => void;
  onPrevious: () => void;
  onNext: (cursor: string) => void;
  onRequestRecentAuth: () => void;
}) {
  return (
    <section className="space-y-3" aria-labelledby="media-reconciliation-title">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h3 id="media-reconciliation-title" className="flex items-center gap-2 font-semibold">
            <ScanSearch className="size-4 text-primary" aria-hidden="true" />媒体一致性检查
          </h3>
          <p className="mt-1 text-sm text-muted-foreground">
            分页检查数据库中的审核、发布、处理与清理状态；每次读取都会写入审计。
          </p>
        </div>
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={isFetching}
          onClick={onRefresh}
        >
          <RefreshCw
            className={`size-4 ${isFetching ? "animate-spin" : ""}`}
            aria-hidden="true"
          />
          {isFetching ? "检查中…" : "从头重新检查"}
        </Button>
      </div>

      <Card className="border-primary/30 bg-primary/5">
        <CardContent className="flex gap-3 p-4 text-sm leading-6">
          <ShieldAlert className="mt-1 size-4 shrink-0 text-primary" aria-hidden="true" />
          <p>
            <strong>只读 dry-run：</strong>
            此检查不会自动修复、重新排队或删除任何对象。下方数量来自 PostgreSQL 候选记录，
            不代表已读取 OSS/CDN 的真实对象清单；provider inventory 必须按运维手册另行人工核对。
          </p>
        </CardContent>
      </Card>

      {isLoading ? <LoadingState label="加载媒体一致性报告" /> : null}
      {hasError && !requiresRecentAuth ? (
        <ErrorState title="媒体一致性检查失败" error={error} onRetry={onRefresh} />
      ) : null}
      {requiresRecentAuth ? (
        <Card>
          <CardContent className="flex flex-col items-start gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <p className="font-medium">查看一致性报告前需要重新验证</p>
              <p className="mt-1 text-sm text-muted-foreground">
                验证后会刷新一致性报告、保留清单与删除任务。
              </p>
            </div>
            <Button type="button" size="sm" onClick={onRequestRecentAuth}>
              重新验证
            </Button>
          </CardContent>
        </Card>
      ) : null}
      {!hasError && report && report.dryRun !== true ? (
        <Card className="border-destructive/30" role="alert">
          <CardContent className="p-4">
            <p className="font-medium text-destructive">服务端未确认只读模式</p>
            <p className="mt-1 text-sm text-muted-foreground">
              为避免误导，当前响应不会作为一致性检查结果展示。
            </p>
          </CardContent>
        </Card>
      ) : null}
      {!hasError && report?.dryRun === true ? (
        <>
          <Card>
            <CardHeader className="pb-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  <CardTitle className="text-sm">Provider 对象清单</CardTitle>
                  <CardDescription className="mt-1">
                    本报告只给出需要与 Ingest、Delivery 私有桶核对的数据库候选数。
                  </CardDescription>
                </div>
                <div className="flex flex-col items-end gap-1">
                  <Badge variant="outline">需要人工核对</Badge>
                  <code className="text-[11px] text-muted-foreground">
                    {report.providerInventory.state}
                  </code>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <dl className="grid gap-3 text-sm sm:grid-cols-2">
                <div className="rounded-lg border bg-muted/30 p-3">
                  <dt className="text-xs text-muted-foreground">Ingest 候选记录</dt>
                  <dd className="mt-1 text-lg font-semibold">
                    {formatNumber(report.providerInventory.ingestCandidateCount)}
                  </dd>
                </div>
                <div className="rounded-lg border bg-muted/30 p-3">
                  <dt className="text-xs text-muted-foreground">Delivery 候选变体</dt>
                  <dd className="mt-1 text-lg font-semibold">
                    {formatNumber(report.providerInventory.deliveryCandidateCount)}
                  </dd>
                </div>
              </dl>
            </CardContent>
          </Card>

          {report.items.length === 0 ? (
            <EmptyState
              title="本页未发现数据库一致性异常"
              description="这不替代 OSS/CDN provider inventory 核对，也不会自动修复其他页面上的异常。"
            />
          ) : (
            <div className="grid gap-3 xl:grid-cols-2">
              {report.items.map((finding) => (
                <Card key={finding.assetId}>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm">资产 #{finding.assetId}</CardTitle>
                    <CardDescription>{finding.issueCodes.length} 项数据库状态异常</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {finding.issueCodes.map((issueCode) => (
                        <li key={issueCode} className="rounded-lg border bg-muted/30 p-3">
                          <code className="break-all text-xs font-medium text-destructive">
                            {issueCode}
                          </code>
                          <p className="mt-1 text-sm">
                            {ISSUE_LABELS[issueCode] ?? "未识别的一致性异常"}
                          </p>
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
          <PaginationControls
            hasPrevious={hasPrevious}
            hasMore={Boolean(report.nextCursor)}
            onPrevious={onPrevious}
            onNext={() => report.nextCursor && onNext(report.nextCursor)}
          />
        </>
      ) : null}
    </section>
  );
}
