import {
  BarChart3,
  Bookmark,
  Copy,
  Eye,
  ExternalLink,
  Loader2,
  MessageCircle,
  MoreHorizontal,
  Share2,
  ThumbsUp,
} from "lucide-react";
import { Link } from "react-router";
import { toast } from "sonner";

import { TeaBadge } from "@/components/common/tea-badge";
import { ForumDeliveryImage } from "@/components/content/forum-delivery-image";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ForumAttachment } from "@/lib/api/types";
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

export interface ProfilePostCardData {
  id: string;
  title: string;
  body?: string | null;
  boardSlug?: string | null;
  createdAtLabel: string;
  replyCount?: number | null;
  voteCount?: number | null;
  viewCount?: number | null;
  heatCount?: number | null;
  attachment?: ForumAttachment | null;
  href?: string | null;
  isBookmarked?: boolean;
}

interface ProfilePostCardProps {
  post: ProfilePostCardData;
  authorName: string;
  authorHandle: string;
  authorAvatarUrl?: string | null;
  trustLevel?: number | null;
  bookmarkPending?: boolean;
  onToggleBookmark?: () => void;
  onAttachmentDeliveryRefresh?: () => void;
  className?: string;
}

function canonicalUrl(href: string) {
  return new URL(href, window.location.origin).toString();
}

async function copyUrl(url: string) {
  if (!navigator.clipboard?.writeText) {
    throw new Error("当前浏览器不支持复制链接");
  }
  await navigator.clipboard.writeText(url);
  toast.success("链接已复制");
}

async function sharePost(title: string, href: string) {
  const url = canonicalUrl(href);
  if (navigator.share) {
    try {
      await navigator.share({ title, url });
      return;
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
    }
  }
  await copyUrl(url);
}

export function ProfilePostCard({
  post,
  authorName,
  authorHandle,
  authorAvatarUrl,
  trustLevel,
  bookmarkPending = false,
  onToggleBookmark,
  onAttachmentDeliveryRefresh,
  className,
}: ProfilePostCardProps) {
  const initial = (authorHandle || authorName || "?").slice(0, 1).toUpperCase();

  const copyPostUrl = async () => {
    if (!post.href) return;
    try {
      await copyUrl(canonicalUrl(post.href));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "复制链接失败");
    }
  };

  const share = async () => {
    if (!post.href) return;
    try {
      await sharePost(post.title, post.href);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "分享失败");
    }
  };

  return (
    <Card
      className={cn(
        "group rounded-2xl border-border/50 bg-card shadow-none transition-colors hover:border-primary/20 hover:bg-[#eef1ef]/70 dark:hover:bg-accent/40",
        className,
      )}
    >
      <CardContent className="px-4 py-3.5">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            <Avatar className="size-10 shrink-0 border-[2.5px] border-primary bg-white p-[2px] shadow-sm">
              <AvatarImage src={authorAvatarUrl ?? undefined} alt="" />
              <AvatarFallback className="bg-primary/10 text-xs font-semibold text-primary">
                {initial}
              </AvatarFallback>
            </Avatar>
            <div className="min-w-0 pt-0.5">
              <div className="flex min-w-0 flex-wrap items-center gap-1.5">
                <span className="truncate text-[14px] font-semibold leading-5 text-foreground">
                  {authorName}
                </span>
                {typeof trustLevel === "number" ? <TeaBadge level={trustLevel} /> : null}
              </div>
              <p className="mt-0.5 text-[12px] leading-4 text-muted-foreground">
                {post.createdAtLabel}
              </p>
            </div>
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8 shrink-0 text-muted-foreground hover:bg-transparent hover:text-foreground"
                aria-label="更多操作"
              >
                <MoreHorizontal className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {post.href ? (
                <DropdownMenuItem asChild>
                  <Link to={post.href}>
                    <ExternalLink className="size-4" />
                    打开内容
                  </Link>
                </DropdownMenuItem>
              ) : null}
              {post.href ? (
                <DropdownMenuItem onSelect={() => void copyPostUrl()}>
                  <Copy className="size-4" />
                  复制链接
                </DropdownMenuItem>
              ) : null}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        <div className="mt-2.5 pl-[52px]">
          {post.href ? (
            <Link
              to={post.href}
              className="block rounded-lg outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
            >
              <h2 className="line-clamp-2 text-[15px] font-semibold leading-6 tracking-tight text-foreground transition-colors group-hover:text-primary">
                {post.title || "未命名主题"}
              </h2>
              {post.body ? (
                <p className="mt-1 line-clamp-3 text-[13px] leading-5 text-muted-foreground">
                  {post.body}
                </p>
              ) : null}
              {post.attachment ? (
                <ForumDeliveryImage
                  attachment={post.attachment}
                  onDeliveryRefresh={onAttachmentDeliveryRefresh}
                  loading="lazy"
                  decoding="async"
                  className="mt-3 max-h-64 w-full rounded-xl border border-border/50 object-cover"
                />
              ) : null}
            </Link>
          ) : (
            <>
              <h2 className="line-clamp-2 text-[15px] font-semibold leading-6 tracking-tight text-foreground">
                {post.title || "未命名主题"}
              </h2>
              {post.body ? (
                <p className="mt-1 line-clamp-3 text-[13px] leading-5 text-muted-foreground">
                  {post.body}
                </p>
              ) : null}
            </>
          )}

          {post.boardSlug ? (
            <div className="mt-3">
              <span className="inline-flex h-[22px] items-center rounded-full bg-[#e9f6ef] px-2.5 text-[11px] font-medium leading-none tracking-[0.01em] text-[#3d8f6b] shadow-[inset_0_0_0_1px_rgba(61,143,107,0.16)] dark:bg-primary/10 dark:text-primary dark:shadow-[inset_0_0_0_1px_rgba(61,143,107,0.28)]">
                {post.boardSlug}
              </span>
            </div>
          ) : null}
        </div>

        <div className="mt-3 flex items-center justify-between gap-3 pl-[52px]">
          <div className="flex min-w-0 flex-wrap items-center gap-x-5 gap-y-1 text-[12px] text-muted-foreground">
            {typeof post.replyCount === "number" ? (
              <span className="inline-flex items-center gap-1.5" aria-label={`${post.replyCount} 条回复`}>
                <MessageCircle className="size-3.5" aria-hidden="true" />
                {formatNumber(post.replyCount)}
              </span>
            ) : null}
            {typeof post.voteCount === "number" ? (
              <span className="inline-flex items-center gap-1.5" aria-label={`${post.voteCount} 个赞`}>
                <ThumbsUp className="size-3.5" aria-hidden="true" />
                {formatNumber(post.voteCount)}
              </span>
            ) : null}
            {typeof post.viewCount === "number" ? (
              <span className="inline-flex items-center gap-1.5">
                <Eye className="size-3.5" aria-hidden="true" />
                {formatNumber(post.viewCount)}
              </span>
            ) : null}
            {typeof post.heatCount === "number" ? (
              <span className="inline-flex items-center gap-1.5">
                <BarChart3 className="size-3.5" aria-hidden="true" />
                {formatNumber(post.heatCount)}
              </span>
            ) : null}
          </div>

          <div className="flex shrink-0 items-center gap-0.5">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className={cn(
                "size-7 hover:bg-transparent hover:text-foreground",
                post.isBookmarked ? "text-primary" : "text-muted-foreground",
              )}
              aria-label={post.isBookmarked ? "取消收藏" : "收藏"}
              disabled={bookmarkPending || !onToggleBookmark}
              onClick={onToggleBookmark}
            >
              {bookmarkPending ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <Bookmark className="size-3.5" fill={post.isBookmarked ? "currentColor" : "none"} />
              )}
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:bg-transparent hover:text-foreground"
              aria-label="分享"
              disabled={!post.href}
              onClick={() => void share()}
            >
              <Share2 className="size-3.5" />
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
