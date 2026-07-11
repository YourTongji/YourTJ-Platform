import { AlertCircle, Check, Loader2, RefreshCw, ShieldCheck } from "lucide-react";
import * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

const DEFAULT_CAPTCHA_URL = "https://captcha.07211024.xyz";

interface CaptchaChallenge {
  puzzleToken: string;
  prompt: string;
  images: string[];
}

interface CaptchaVerifyResponse {
  success?: boolean;
  token?: string;
  message?: string;
}

function captchaBaseUrl() {
  return (import.meta.env.VITE_CAPTCHA_URL ?? DEFAULT_CAPTCHA_URL).replace(/\/+$/, "");
}

function captchaUrl(path: string) {
  return new URL(path, `${captchaBaseUrl()}/`).toString();
}

function parseChallenge(value: unknown): CaptchaChallenge {
  if (!value || typeof value !== "object") {
    throw new Error("验证码服务返回了无效挑战");
  }
  const challenge = value as {
    puzzle_token?: unknown;
    prompt?: unknown;
    questionType?: unknown;
    images?: unknown;
  };
  if (
    typeof challenge.puzzle_token !== "string"
    || !Array.isArray(challenge.images)
    || challenge.images.length === 0
    || challenge.images.some((image) => typeof image !== "string")
  ) {
    throw new Error("验证码服务返回了不完整挑战");
  }
  const fallbackPrompt = challenge.questionType === "TONGJI_NOT_IN"
    ? "选择下列不在同济校内的图片："
    : "选择下列在同济校内的图片：";
  return {
    puzzleToken: challenge.puzzle_token,
    prompt: typeof challenge.prompt === "string" && challenge.prompt.trim()
      ? challenge.prompt
      : fallbackPrompt,
    images: challenge.images.map((image) => captchaUrl(image as string)),
  };
}

export function YourTJCaptcha({
  open,
  onOpenChange,
  onVerified,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onVerified: (token: string) => void;
}) {
  const [challenge, setChallenge] = React.useState<CaptchaChallenge | null>(null);
  const [selected, setSelected] = React.useState<Set<number>>(() => new Set());
  const [isLoading, setIsLoading] = React.useState(false);
  const [isVerifying, setIsVerifying] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const requestController = React.useRef<AbortController | null>(null);

  const loadChallenge = React.useCallback(async (statusMessage?: string) => {
    requestController.current?.abort();
    const controller = new AbortController();
    requestController.current = controller;
    setChallenge(null);
    setSelected(new Set());
    setError(statusMessage ?? null);
    setIsLoading(true);
    try {
      const response = await fetch(captchaUrl("/api/captcha"), {
        cache: "no-store",
        credentials: "omit",
        headers: { Accept: "application/json" },
        signal: controller.signal,
      });
      if (!response.ok) {
        throw new Error("验证码加载失败，请稍后重试");
      }
      setChallenge(parseChallenge(await response.json()));
    } catch (loadError) {
      if (controller.signal.aborted) return;
      const isInvalidChallenge = loadError instanceof Error
        && loadError.message.startsWith("验证码服务返回");
      setError(isInvalidChallenge ? loadError.message : "验证码加载失败，请检查网络后重试");
    } finally {
      if (!controller.signal.aborted) setIsLoading(false);
    }
  }, []);

  React.useEffect(() => {
    if (open) {
      void loadChallenge();
    } else {
      requestController.current?.abort();
      setChallenge(null);
      setSelected(new Set());
      setError(null);
      setIsLoading(false);
      setIsVerifying(false);
    }
    return () => requestController.current?.abort();
  }, [loadChallenge, open]);

  function toggleSelection(index: number) {
    if (isVerifying) return;
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  }

  async function verify() {
    if (!challenge || isVerifying) return;
    setIsVerifying(true);
    setError(null);
    try {
      const response = await fetch(captchaUrl("/api/verify"), {
        method: "POST",
        cache: "no-store",
        credentials: "omit",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          puzzle_token: challenge.puzzleToken,
          selected_indices: [...selected].sort((left, right) => left - right),
        }),
      });
      const result = await response.json() as CaptchaVerifyResponse;
      if (!response.ok || !result.success || !result.token) {
        await loadChallenge(result.message || "选择不正确，已为你刷新验证码");
        return;
      }
      onVerified(result.token);
    } catch {
      setError("验证请求失败，请稍后重试");
    } finally {
      setIsVerifying(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => !isVerifying && onOpenChange(nextOpen)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldCheck className="size-5 text-primary" aria-hidden="true" />
            完成人机验证
          </DialogTitle>
          <DialogDescription>
            按提示选择图片。验证令牌只用于当前操作，不会用于识别你的公开身份。
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-72" aria-busy={isLoading || isVerifying}>
          {isLoading ? (
            <div className="flex min-h-72 items-center justify-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-5 animate-spin" aria-hidden="true" />
              正在加载真实验证码图片
            </div>
          ) : error && !challenge ? (
            <div className="flex min-h-72 flex-col items-center justify-center gap-3 text-center" role="alert">
              <AlertCircle className="size-6 text-destructive" aria-hidden="true" />
              <p className="max-w-sm text-sm text-muted-foreground">{error}</p>
              <Button type="button" variant="outline" onClick={() => void loadChallenge()}>
                <RefreshCw className="size-4" />重试
              </Button>
            </div>
          ) : challenge ? (
            <div className="space-y-4">
              <p className="text-sm font-medium" id="yourtj-captcha-prompt">{challenge.prompt}</p>
              <div
                className="grid grid-cols-3 gap-2"
                role="group"
                aria-labelledby="yourtj-captcha-prompt"
              >
                {challenge.images.map((image, index) => {
                  const isSelected = selected.has(index);
                  return (
                    <button
                      key={`${challenge.puzzleToken}-${index}`}
                      type="button"
                      aria-label={`验证码图片选项 ${index + 1}${isSelected ? "，已选择" : ""}`}
                      aria-pressed={isSelected}
                      onClick={() => toggleSelection(index)}
                      disabled={isVerifying}
                      className={cn(
                        "relative aspect-square overflow-hidden rounded-lg border-2 bg-muted outline-none transition focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-wait",
                        isSelected ? "border-primary shadow-sm" : "border-transparent hover:border-primary/40",
                      )}
                    >
                      <img
                        src={image}
                        alt=""
                        draggable={false}
                        className="size-full object-cover"
                      />
                      {isSelected ? (
                        <span className="absolute inset-0 flex items-center justify-center bg-primary/25" aria-hidden="true">
                          <span className="flex size-8 items-center justify-center rounded-full bg-primary text-primary-foreground shadow">
                            <Check className="size-5" />
                          </span>
                        </span>
                      ) : null}
                    </button>
                  );
                })}
              </div>
              {error ? <p className="text-sm text-destructive" role="alert">{error}</p> : null}
              <p className="text-xs text-muted-foreground" aria-live="polite">
                已选择 {selected.size} 张图片；如果没有符合条件的图片，可以直接提交。
              </p>
            </div>
          ) : null}
        </div>

        <DialogFooter className="sm:justify-between">
          <Button
            type="button"
            variant="ghost"
            onClick={() => void loadChallenge()}
            disabled={isLoading || isVerifying}
          >
            <RefreshCw className="size-4" />换一组
          </Button>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isVerifying}>
              取消
            </Button>
            <Button type="button" onClick={() => void verify()} disabled={!challenge || isLoading || isVerifying}>
              {isVerifying ? <Loader2 className="size-4 animate-spin" /> : <ShieldCheck className="size-4" />}
              {isVerifying ? "正在验证" : "提交验证"}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
