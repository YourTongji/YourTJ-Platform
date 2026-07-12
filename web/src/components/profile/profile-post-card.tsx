import {
  BarChart3,
  Bookmark,
  Eye,
  MessageCircle,
  MoreHorizontal,
  Share2,
  ThumbsUp,
} from "lucide-react";
import { Link } from "react-router";

import { TeaBadge } from "@/components/common/tea-badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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
  imageUrl?: string | null;
  href?: string | null;
}

interface ProfilePostCardProps {
  post: ProfilePostCardData;
  authorName: string;
  authorHandle: string;
  authorAvatarUrl?: string | null;
  trustLevel?: number | null;
  className?: string;
}

/**
 * Figma 个人主页信息流卡片（对齐社区信息流）：
 * 头像 + 昵称/茶等级 + 更多
 * 时间
 * 标题 / 摘要 / 可选配图
 * 板块标签
 * 评论 · 点赞 · 浏览 · 热度 | 收藏 · 分享
 */
export function ProfilePostCard({
  post,
  authorName,
  authorHandle,
  authorAvatarUrl,
  trustLevel,
  className,
}: ProfilePostCardProps) {
  const initial = (authorHandle || authorName || "?").slice(0, 1).toUpperCase();

  const body = (
    <>
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
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-8 shrink-0 text-muted-foreground hover:bg-transparent hover:text-foreground"
          aria-label="更多操作"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
          }}
        >
          <MoreHorizontal className="size-4" />
        </Button>
      </div>

      <div className="mt-2.5 pl-[52px]">
        <h2 className="line-clamp-2 text-[15px] font-semibold leading-6 tracking-tight text-foreground transition-colors group-hover:text-primary">
          {post.title || "未命名主题"}
        </h2>

        {post.body ? (
          <p className="mt-1 line-clamp-3 text-[13px] leading-5 text-muted-foreground">
            {post.body}
          </p>
        ) : null}

        {post.imageUrl ? (
          <img
            src={post.imageUrl}
            alt=""
            loading="lazy"
            decoding="async"
            className="mt-3 max-h-64 w-full rounded-xl border border-border/50 object-cover"
          />
        ) : null}

        {post.boardSlug ? (
          <div className="mt-3">
            {/* Figma 首页信息流标签：正文下方浅薄荷绿胶囊，弱描边、小字号 */}
            <span className="inline-flex h-[22px] items-center rounded-full bg-[#e9f6ef] px-2.5 text-[11px] font-medium leading-none tracking-[0.01em] text-[#3d8f6b] shadow-[inset_0_0_0_1px_rgba(61,143,107,0.16)] dark:bg-primary/10 dark:text-primary dark:shadow-[inset_0_0_0_1px_rgba(61,143,107,0.28)]">
              {post.boardSlug}
            </span>
          </div>
        ) : null}
      </div>
    </>
  );

  return (
    <Card
      className={cn(
        "group rounded-2xl border-border/50 bg-card shadow-none transition-colors hover:border-primary/20 hover:bg-[#eef1ef]/70 dark:hover:bg-accent/40",
        className,
      )}
    >
      <CardContent className="px-4 py-3.5">
        {post.href ? (
          <Link
            to={post.href}
            className="block rounded-lg outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
          >
            {body}
          </Link>
        ) : (
          <div>{body}</div>
        )}

        <div className="mt-3 flex items-center justify-between gap-3 pl-[52px]">
          <div className="flex min-w-0 flex-wrap items-center gap-x-5 gap-y-1 text-[12px] text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <MessageCircle className="size-3.5" aria-hidden="true" />
              {formatNumber(post.replyCount ?? 0)}
            </span>
            <span className="inline-flex items-center gap-1.5">
              <ThumbsUp className="size-3.5" aria-hidden="true" />
              {formatNumber(post.voteCount ?? 0)}
            </span>
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
              className="size-7 text-muted-foreground hover:bg-transparent hover:text-foreground"
              aria-label="收藏"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
              }}
            >
              <Bookmark className="size-3.5" />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-7 text-muted-foreground hover:bg-transparent hover:text-foreground"
              aria-label="分享"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
              }}
            >
              <Share2 className="size-3.5" />
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
