import {
  Bell,
  Bookmark,
  BookOpen,
  CalendarDays,
  CircleHelp,
  Home,
  MessageSquare,
  Settings,
  Shield,
  WalletCards,
} from "lucide-react";
import { Link, NavLink } from "react-router";

import { useAuth } from "@/context/auth-provider";
import { cn } from "@/lib/utils";

const primaryNavigation = [
  { to: "/", label: "首页", icon: Home },
  { to: "/forum", label: "社区", icon: MessageSquare },
  { to: "/schedule", label: "选课排课", icon: CalendarDays },
  { to: "/courses", label: "课程评课", icon: BookOpen },
  { to: "/wallet", label: "积分任务", icon: WalletCards },
];

const communityNavigation = [
  { to: "/notifications", label: "社区公告", icon: Bell },
  { to: "/messages", label: "私信", icon: MessageSquare },
  { to: "/bookmarks", label: "收藏", icon: Bookmark },
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
        <img src="/icon.png" alt="YourTJ" className="size-full object-cover" />
      </span>
      <span
        className={cn(
          "font-display font-bold text-[#1a1a1a] dark:text-foreground",
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
  onNavigate,
}: {
  items: typeof primaryNavigation;
  onNavigate?: () => void;
}) {
  return (
    <nav className="space-y-2">
      {items.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === "/"}
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              "flex h-10 items-center gap-3 rounded-lg px-3 text-base font-medium transition-colors",
              isActive
                ? "bg-[#f3f4f6] text-primary dark:bg-secondary"
                : "text-[#6b7280] hover:bg-[#f3f4f6] hover:text-foreground dark:text-muted-foreground dark:hover:bg-accent",
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

function PromotionSlot() {
  return (
    <div className="rounded-lg border border-primary/20 bg-primary/10 px-3 py-3 text-center">
      <p className="text-[10px] font-medium tracking-[0.1em] text-primary">
        ADVERTISEMENT
      </p>
      <p className="mt-1 text-xs text-[#374151] dark:text-foreground">
        社区推广位
      </p>
    </div>
  );
}

export function SiteSidebar({ onNavigate }: { onNavigate?: () => void }) {
  const { account } = useAuth();
  const secondaryItems =
    account?.role === "admin" || account?.role === "mod"
      ? [...communityNavigation, adminNavigation]
      : communityNavigation;

  return (
    <div className="flex min-h-full flex-col pb-8 pr-6 pt-2">
      <NavigationGroup items={primaryNavigation} onNavigate={onNavigate} />

      <div className="my-4 border-t" />
      <NavigationGroup items={secondaryItems} onNavigate={onNavigate} />

      <div className="mt-4 space-y-4 px-3">
        <PromotionSlot />
        <PromotionSlot />
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
