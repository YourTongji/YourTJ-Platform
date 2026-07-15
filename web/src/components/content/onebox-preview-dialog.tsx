import { Link2, Loader2 } from "lucide-react";
import * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api/endpoints";
import type { OneboxResult } from "@/lib/api/types";

function validatePreviewUrl(value: string) {
  try {
    const url = new URL(value.trim());
    if (url.protocol !== "https:" || (url.port && url.port !== "443")) {
      return "只支持使用标准 HTTPS 端口的链接";
    }
    return undefined;
  } catch {
    return "请输入完整的 HTTPS 链接";
  }
}

export function OneboxPreviewDialog({
  disabled = false,
  onInsert,
}: {
  disabled?: boolean;
  onInsert: (url: string, label: string) => void;
}) {
  const [open, setOpen] = React.useState(false);
  const [url, setUrl] = React.useState("");
  const [result, setResult] = React.useState<OneboxResult | null>(null);
  const [error, setError] = React.useState<string>();
  const [isLoading, setIsLoading] = React.useState(false);
  const controllerRef = React.useRef<AbortController | null>(null);

  React.useEffect(() => () => controllerRef.current?.abort(), []);

  function clearRequest() {
    controllerRef.current?.abort();
    controllerRef.current = null;
    setIsLoading(false);
  }

  function changeOpen(nextOpen: boolean) {
    setOpen(nextOpen);
    if (!nextOpen) {
      clearRequest();
      setUrl("");
      setResult(null);
      setError(undefined);
    }
  }

  async function preview() {
    const requestedUrl = url.trim();
    const validationError = validatePreviewUrl(requestedUrl);
    if (validationError) {
      setResult(null);
      setError(validationError);
      return;
    }
    clearRequest();
    const controller = new AbortController();
    controllerRef.current = controller;
    setIsLoading(true);
    setResult(null);
    setError(undefined);
    try {
      const previewResult = await api.onebox(requestedUrl, controller.signal);
      if (!controller.signal.aborted && controllerRef.current === controller) {
        setResult(previewResult);
      }
    } catch (requestError) {
      if (!controller.signal.aborted && controllerRef.current === controller) {
        setError(requestError instanceof Error ? requestError.message : "链接预览失败");
      }
    } finally {
      if (controllerRef.current === controller) {
        controllerRef.current = null;
        setIsLoading(false);
      }
    }
  }

  const insertLabel = result?.title?.trim() || result?.siteName?.trim() || "链接";

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-8"
          disabled={disabled}
          aria-label="预览并插入链接"
        >
          <Link2 className="size-4" />
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>预览链接</DialogTitle>
          <DialogDescription>
            只有点击“安全预览”后，平台服务器才会读取允许站点的公开文字元数据；浏览器不会直连目标站或加载远程图片。
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <label htmlFor="onebox-preview-url" className="text-sm font-medium">HTTPS 链接</label>
          <div className="flex gap-2">
            <Input
              id="onebox-preview-url"
              type="url"
              inputMode="url"
              autoComplete="off"
              maxLength={2048}
              value={url}
              onChange={(event) => {
                clearRequest();
                setUrl(event.target.value);
                setResult(null);
                setError(undefined);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  if (!isLoading) void preview();
                }
              }}
              placeholder="https://example.edu/page"
              aria-describedby={error ? "onebox-preview-error" : "onebox-preview-help"}
            />
            <Button type="button" variant="outline" onClick={() => void preview()} disabled={isLoading || !url.trim()}>
              {isLoading ? <Loader2 className="size-4 animate-spin" /> : null}
              安全预览
            </Button>
          </div>
          {error ? (
            <p id="onebox-preview-error" role="alert" className="text-sm text-destructive">{error}</p>
          ) : (
            <p id="onebox-preview-help" className="text-xs text-muted-foreground">
              输入值只保存在当前对话框内，不写入页面地址或浏览器持久存储。
            </p>
          )}
          {result ? (
            <div className="rounded-lg border bg-muted/20 p-4" aria-live="polite">
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                {result.type === "card" ? result.siteName || "链接预览" : "普通链接"}
              </p>
              <p className="mt-1 font-semibold">{result.title || "没有可用标题"}</p>
              {result.description ? (
                <p className="mt-1 line-clamp-3 text-sm text-muted-foreground">{result.description}</p>
              ) : null}
              {result.type === "plain" ? (
                <p className="mt-2 text-xs text-muted-foreground">该站点不在文字预览允许列表中，仍可作为普通链接插入。</p>
              ) : null}
            </div>
          ) : null}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => changeOpen(false)}>取消</Button>
          <Button
            type="button"
            disabled={!result}
            onClick={() => {
              if (!result) return;
              onInsert(result.url, insertLabel);
              changeOpen(false);
            }}
          >
            插入预览链接
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
