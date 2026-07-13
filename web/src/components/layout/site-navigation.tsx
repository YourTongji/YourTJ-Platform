import {
  Bell,
  BellRing,
  Bookmark,
  BookOpen,
  CalendarDays,
  CircleHelp,
  Home,
  MessageSquare,
  Settings,
  Shield,
  Scale,
  Sparkles,
  WalletCards,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import * as React from "react";
import { Link, NavLink } from "react-router";

import { capabilitiesForAccount } from "@/components/admin/capabilities";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Promotion } from "@/lib/api/types";
import { cn } from "@/lib/utils";

const primaryNavigation = [
  { to: "/", label: "首页", icon: Home },
  { to: "/forum", label: "社区", icon: MessageSquare },
  { to: "/schedule", label: "选课排课", icon: CalendarDays },
  { to: "/courses", label: "课程评课", icon: BookOpen },
  { to: "/wallet", label: "积分任务", icon: WalletCards },
];

const communityNavigation = [
  { to: "/announcements", label: "社区公告", icon: BellRing },
  { to: "/notifications", label: "通知", icon: Bell },
  { to: "/messages", label: "私信", icon: MessageSquare },
  { to: "/bookmarks", label: "收藏", icon: Bookmark },
  { to: "/appeals", label: "申诉中心", icon: Scale },
];

const adminNavigation = { to: "/admin", label: "社区管理", icon: Shield };

export function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <Link
      to="/"
      className="flex shrink-0 items-center gap-2"
      aria-label="YourTJ 社区首页"
    >
      <span className="flex size-8 items-center justify-center rounded-full overflow-hidden">
        <img
          src={`${import.meta.env.BASE_URL}icon.png`}
          alt="YourTJ"
          className="size-full object-cover"
        />
      </span>
      <span
        className={cn(
          "font-sans font-bold text-foreground",
          compact ? "text-base" : "hidden text-xl sm:inline",
        )}
      >
        YourTJ 社区
      </span>
    </Link>
  );
}

function NavigationGroup({
  items,
  label,
  onNavigate,
}: {
  items: typeof primaryNavigation;
  label: string;
  onNavigate?: () => void;
}) {
  return (
    <nav className="space-y-2" aria-label={label}>
      {items.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === "/"}
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              "motion-interactive flex h-10 items-center gap-3 rounded-lg px-3 text-base font-medium",
              isActive
                ? "bg-muted text-primary dark:bg-secondary"
                : "text-[#3d4947] hover:bg-muted hover:text-foreground dark:text-muted-foreground dark:hover:bg-accent",
            )
          }
        >
          <item.icon className="size-5 shrink-0" strokeWidth={1.7} />
          <span>{item.label}</span>
        </NavLink>
      ))}
    </nav>
  );
}

function PromotionAsset({
  promotion,
  onDeliveryError,
}: {
  promotion: Promotion;
  onDeliveryError: () => void;
}) {
  if (!promotion.assetDelivery?.url) {
    return null;
  }
  return (
    <img
      src={promotion.assetDelivery.url}
      alt=""
      width={promotion.assetDelivery.width}
      height={promotion.assetDelivery.height}
      referrerPolicy="no-referrer"
      onError={onDeliveryError}
      className="mb-3 aspect-[16/7] w-full rounded-md object-cover"
    />
  );
}

function PromotionSlot({
  promotion,
  onNavigate,
  onDeliveryError,
}: {
  promotion: Promotion;
  onNavigate?: () => void;
  onDeliveryError: () => void;
}) {
  const linkRef = React.useRef<HTMLAnchorElement>(null);
  const reportedToken = React.useRef<string | null>(null);

  React.useEffect(() => {
    const token = promotion.trackingToken;
    const node = linkRef.current;
    if (!token || !node || reportedToken.current === token) return;
    const record = () => {
      if (reportedToken.current === token) return;
      reportedToken.current = token;
      void api.recordPromotionEvent(promotion.id, "impression", token).catch(() => undefined);
    };
    if (!("IntersectionObserver" in window)) return;
    let visibilityTimer: number | undefined;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (reportedToken.current === token) return;
        if (entry?.isIntersecting && entry.intersectionRatio >= 0.5) {
          if (visibilityTimer === undefined) {
            visibilityTimer = window.setTimeout(record, 500);
          }
        } else if (visibilityTimer !== undefined) {
          window.clearTimeout(visibilityTimer);
          visibilityTimer = undefined;
        }
      },
      { threshold: 0.5 },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
      if (visibilityTimer !== undefined) window.clearTimeout(visibilityTimer);
    };
  }, [promotion.id, promotion.trackingToken]);

  return (
    <Link
      ref={linkRef}
      to={promotion.targetUrl}
      onClick={() => {
        if (promotion.trackingToken) {
          void api.recordPromotionEvent(
            promotion.id,
            "click",
            promotion.trackingToken,
          ).catch(() => undefined);
        }
        onNavigate?.();
      }}
      className="motion-interactive block rounded-lg border border-primary/25 bg-primary/10 p-3 text-left outline-none hover:border-primary/45 hover:bg-primary/15 focus-visible:ring-[3px] focus-visible:ring-ring/50"
    >
      <PromotionAsset promotion={promotion} onDeliveryError={onDeliveryError} />
      <p className="flex items-center gap-1 text-[10px] font-medium tracking-[0.08em] text-primary">
        <Sparkles className="size-3" aria-hidden="true" />
        社区推广
      </p>
      <p className="mt-1 text-sm font-semibold text-foreground">{promotion.title}</p>
      {promotion.body ? (
        <p className="mt-1 line-clamp-3 text-xs leading-5 text-muted-foreground">{promotion.body}</p>
      ) : null}
      <span className="mt-2 inline-block text-xs font-medium text-primary">
        {promotion.ctaLabel || "了解更多"}
      </span>
    </Link>
  );
}

export function SiteSidebar({ onNavigate }: { onNavigate?: () => void }) {
  const { account } = useAuth();
  const promotions = useQuery({
    queryKey: ["promotions", account?.id],
    queryFn: () => api.promotions(),
    staleTime: 60_000,
    refetchInterval: 4 * 60_000,
  });
  const hasStaffCapabilities = capabilitiesForAccount(account).size > 0;
  const secondaryItems =
    hasStaffCapabilities
      ? [...communityNavigation, adminNavigation]
      : communityNavigation;
  const visiblePromotions = promotions.data?.filter(
    (promotion, index, items) => items.findIndex(
      (candidate) => candidate.placement === promotion.placement,
    ) === index,
  );

  return (
    <div className="flex min-h-full flex-col pb-8 pr-6 pt-2">
      <NavigationGroup items={primaryNavigation} label="主要导航" onNavigate={onNavigate} />

      <div className="my-4 border-t border-border/40" />
      <NavigationGroup items={secondaryItems} label="社区与账号" onNavigate={onNavigate} />

      <div className="mt-4 space-y-4 px-3">
        {promotions.isLoading ? (
          <p className="text-xs text-muted-foreground" role="status">正在加载社区推荐…</p>
        ) : promotions.isError ? (
          <p className="text-xs text-muted-foreground" role="status">社区推荐暂不可用</p>
        ) : (
          visiblePromotions?.map((promotion) => (
            <PromotionSlot
              key={promotion.id}
              promotion={promotion}
              onNavigate={onNavigate}
              onDeliveryError={() => void promotions.refetch()}
            />
          ))
        )}
      </div>

      <div className="mt-6 space-y-2 px-3">
        <Link
          to="/settings"
          onClick={onNavigate}
          className="flex h-9 items-center gap-3 rounded-lg px-3 text-sm text-[#6b7280] transition-colors hover:bg-[#f3f4f6] hover:text-foreground"
        >
          <Settings className="size-4" />
          设置
        </Link>
        <a
          href="mailto:help@yourtj.de"
          className="flex h-9 items-center gap-3 rounded-lg px-3 text-sm text-[#6b7280] transition-colors hover:bg-[#f3f4f6] hover:text-foreground"
        >
          <CircleHelp className="size-4" />
          帮助
        </a>
      </div>

      <footer
        id="site-footer"
        className="mt-6 px-3 text-xs leading-4 text-[#9ca3af]"
      >
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-primary">
          <a href="mailto:hello@yourtj.de">关于我们</a>
          <Link to="/forum">社区规则</Link>
          <Link to="/wallet">公开账本</Link>
          <Link to="/forum">团队动态</Link>
          <Link to="/settings">隐私政策</Link>
        </div>
        <div className="mt-4 space-y-1">
          <p>违法和不良信息举报电话：xxxxxx</p>
          <p>举报邮箱：jubao@yourtj.de</p>
          <p className="pt-1">梅李（上海）科技有限公司</p>
          <p>YourTJ Community.</p>
          <p>© 2026 All Rights Reserved.</p>
        </div>
      </footer>
    </div>
  );
}
