import { useQuery } from "@tanstack/react-query";
import { Bell, LogOut, Menu, MessageCircle, Moon, Plus, Search, Settings, Sun, User } from "lucide-react";
import * as React from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router";

import { SearchDialog } from "@/components/layout/search-dialog";
import { RealtimeRefresh } from "@/components/notifications/realtime-refresh";
import { Brand, SiteSidebar } from "@/components/layout/site-navigation";
import { AnnouncementModalQueue } from "@/components/announcements/announcement-modal-queue";
import { PageTransition } from "@/components/common/page-transition";
import { RouteLoadingState } from "@/components/common/route-loading-state";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { cn } from "@/lib/utils";

function ThemeToggle() {
  const [isDark, setIsDark] = React.useState(() => document.documentElement.classList.contains("dark"));

  return (
    <Button
      variant="ghost"
      size="icon"
      className="size-9 rounded-full text-[#6b7280]"
      onClick={() => {
        document.documentElement.classList.toggle("dark", !isDark);
        localStorage.setItem("yourtj.theme", !isDark ? "dark" : "light");
        setIsDark(!isDark);
      }}
      aria-label="切换主题"
    >
      {isDark ? (
        <Sun key="sun" className="motion-pop size-[18px]" />
      ) : (
        <Moon key="moon" className="motion-pop size-[18px]" />
      )}
    </Button>
  );
}

export function AppLayout() {
  const [searchOpen, setSearchOpen] = React.useState(false);
  const [mobileOpen, setMobileOpen] = React.useState(false);
  const { account, isAuthenticated, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const isHome = location.pathname === "/";
  const notificationCount = useQuery({
    queryKey: ["notification-count"],
    queryFn: api.unreadNotificationCount,
    enabled: isAuthenticated,
    staleTime: 30_000,
    refetchInterval: 60_000,
  });
  const unreadCount = notificationCount.data?.count ?? 0;
  const dmCount = useQuery({
    queryKey: ["dm-unread-count"],
    queryFn: api.dmUnreadCount,
    enabled: isAuthenticated,
    staleTime: 30_000,
    refetchInterval: 60_000,
  });
  const unreadDmCount = dmCount.data?.count ?? 0;

  return (
    <TooltipProvider>
      <RealtimeRefresh isAuthenticated={isAuthenticated} />
      <div className="min-h-screen bg-background">
        <header className="sticky top-0 z-40 h-16 border-b border-border/60 bg-white/95 backdrop-blur dark:bg-card/95">
          <div className="relative mx-auto flex h-full max-w-[1280px] items-center gap-3 px-4 sm:px-6">
            <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
              <SheetTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="min-[1240px]:hidden"
                  aria-label="打开导航"
                >
                  <Menu className="size-5" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="w-[288px] overflow-y-auto px-5 pt-6">
                <Brand compact />
                <div className="mt-6">
                  <SiteSidebar onNavigate={() => setMobileOpen(false)} />
                </div>
              </SheetContent>
            </Sheet>

            <Brand />

            <div className="absolute left-[calc(50%-48px)] hidden w-[448px] -translate-x-1/2 lg:block">
              <button
                type="button"
                onClick={() => setSearchOpen(true)}
                className="motion-interactive flex h-[34px] w-full items-center rounded-full border border-input bg-card px-3 text-left text-sm text-muted-foreground hover:border-primary/40 hover:bg-muted dark:bg-background dark:hover:bg-accent"
              >
                <Search className="mr-2 size-[18px] shrink-0" />
                <span className="truncate">搜索帖子、课程、资料、WIKI...</span>
                <span className="ml-auto rounded border px-1.5 text-[10px] leading-4">/</span>
              </button>
            </div>

            <div className="ml-auto flex items-center gap-1 sm:gap-3 lg:gap-4">
              <Button
                variant="ghost"
                size="icon"
                className="lg:hidden"
                onClick={() => setSearchOpen(true)}
                aria-label="搜索"
              >
                <Search className="size-[18px]" />
              </Button>
              <Button asChild size="sm" className="hidden rounded-full px-4 lg:inline-flex">
                <Link to="/forum">
                  <Plus className="size-3.5" />
                  快速发帖
                </Link>
              </Button>
              <Button asChild variant="ghost" size="icon" className="size-9 rounded-full text-[#6b7280]">
                <Link
                  to="/notifications"
                  className="relative"
                  aria-label={unreadCount > 0 ? `通知，${unreadCount} 条未读` : "通知"}
                >
                  <Bell className="size-[18px]" />
                  {unreadCount > 0 ? (
                    <span className="absolute -right-1 -top-1 min-w-4 rounded-full bg-primary px-1 text-center text-[10px] font-semibold leading-4 text-primary-foreground">
                      {unreadCount > 99 ? "99+" : unreadCount}
                    </span>
                  ) : null}
                </Link>
              </Button>
              {isAuthenticated ? (
                <Button asChild variant="ghost" size="icon" className="size-9 rounded-full text-[#6b7280]">
                  <Link
                    to="/messages"
                    className="relative"
                    aria-label={unreadDmCount > 0 ? `私信，${unreadDmCount} 条未读` : "私信"}
                  >
                    <MessageCircle className="size-[18px]" />
                    {unreadDmCount > 0 ? (
                      <span className="absolute -right-1 -top-1 min-w-4 rounded-full bg-primary px-1 text-center text-[10px] font-semibold leading-4 text-primary-foreground">
                        {unreadDmCount > 99 ? "99+" : unreadDmCount}
                      </span>
                    ) : null}
                  </Link>
                </Button>
              ) : null}
              <ThemeToggle />
              {isAuthenticated ? (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button className="motion-interactive rounded-full border bg-card p-1 focus-visible:ring-[3px] focus-visible:ring-ring/50">
                      <Avatar className="size-8">
                        <AvatarImage src={account?.avatarUrl ?? undefined} />
                        <AvatarFallback>{account?.handle?.slice(0, 1).toUpperCase() ?? "我"}</AvatarFallback>
                      </Avatar>
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-52">
                    <DropdownMenuLabel>
                      <p>{account?.handle}</p>
                      <p className="text-xs font-normal text-muted-foreground">{account?.role ?? "user"}</p>
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onSelect={() => navigate(`/profile/${account?.handle}`)}>
                      <User className="size-4" />
                      个人主页
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => navigate("/settings")}>
                      <Settings className="size-4" />
                      设置
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onSelect={() => void logout()}>
                      <LogOut className="size-4" />
                      退出登录
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : (
                <Button asChild size="sm" variant="outline" className="rounded-full px-4">
                  <Link to="/login">登录</Link>
                </Button>
              )}
            </div>
          </div>
        </header>

        <div className="mx-auto max-w-[1280px] px-4 sm:px-6 min-[1360px]:!px-8">
          <div className="min-[1240px]:grid min-[1240px]:grid-cols-[256px_minmax(0,1fr)]">
            <aside className="scrollbar-none sticky top-16 hidden h-[calc(100vh-4rem)] overflow-y-auto border-r min-[1240px]:block">
              <SiteSidebar />
            </aside>
            <main className={cn("min-w-0", !isHome && "px-1 py-6 sm:px-4 lg:px-8")}>
              <React.Suspense fallback={<RouteLoadingState />}>
                <PageTransition>
                  <Outlet />
                </PageTransition>
              </React.Suspense>
            </main>
          </div>
        </div>
      </div>
      <SearchDialog open={searchOpen} onOpenChange={setSearchOpen} />
      <AnnouncementModalQueue />
    </TooltipProvider>
  );
}
