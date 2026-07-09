import {
  Bell,
  BookOpen,
  Bookmark,
  CalendarDays,
  Home,
  LogOut,
  Menu,
  MessageSquare,
  Moon,
  Search,
  Settings,
  Shield,
  Sun,
  User,
  Wallet,
} from "lucide-react";
import * as React from "react";
import { Link, NavLink, Outlet, useNavigate } from "react-router";

import { SearchDialog } from "@/components/layout/search-dialog";
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
import { cn } from "@/lib/utils";

const navigation = [
  { to: "/", label: "首页", icon: Home },
  { to: "/forum", label: "论坛", icon: MessageSquare },
  { to: "/courses", label: "课程点评", icon: BookOpen },
  { to: "/schedule", label: "选课排课", icon: CalendarDays },
  { to: "/wallet", label: "积分钱包", icon: Wallet },
  { to: "/notifications", label: "通知", icon: Bell },
  { to: "/messages", label: "私信", icon: MessageSquare },
  { to: "/bookmarks", label: "收藏", icon: Bookmark },
];

const adminNavigation = { to: "/admin", label: "管理", icon: Shield };

function NavItems({ onNavigate }: { onNavigate?: () => void }) {
  const { account } = useAuth();
  const items =
    account?.role === "admin" || account?.role === "mod"
      ? [...navigation, adminNavigation]
      : navigation;
  return (
    <nav className="space-y-1">
      {items.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
              isActive ? "bg-secondary text-primary" : "text-muted-foreground hover:bg-accent hover:text-foreground",
            )
          }
        >
          <item.icon className="h-4 w-4" />
          {item.label}
        </NavLink>
      ))}
    </nav>
  );
}

function ThemeToggle() {
  const [dark, setDark] = React.useState(() => document.documentElement.classList.contains("dark"));
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={() => {
        document.documentElement.classList.toggle("dark", !dark);
        localStorage.setItem("yourtj.theme", !dark ? "dark" : "light");
        setDark(!dark);
      }}
      aria-label="切换主题"
    >
      {dark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
    </Button>
  );
}

export function AppLayout() {
  const [searchOpen, setSearchOpen] = React.useState(false);
  const [mobileOpen, setMobileOpen] = React.useState(false);
  const { account, isAuthenticated, logout } = useAuth();
  const navigate = useNavigate();

  return (
    <TooltipProvider>
      <div className="min-h-screen bg-background">
        <header className="sticky top-0 z-40 border-b bg-card/90 backdrop-blur">
          <div className="mx-auto flex h-14 max-w-7xl items-center gap-3 px-4">
            <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="lg:hidden" aria-label="打开导航">
                  <Menu className="h-5 w-5" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="w-72">
                <Link to="/" className="mb-6 flex items-center gap-2" onClick={() => setMobileOpen(false)}>
                  <div className="flex h-8 w-8 items-center justify-center rounded-md bg-primary text-primary-foreground">
                    TJ
                  </div>
                  <div>
                    <p className="font-semibold leading-none">你济社区</p>
                    <p className="text-xs text-muted-foreground">YourTJ Platform</p>
                  </div>
                </Link>
                <NavItems onNavigate={() => setMobileOpen(false)} />
              </SheetContent>
            </Sheet>

            <Link to="/" className="flex shrink-0 items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-md bg-primary text-sm font-semibold text-primary-foreground">
                TJ
              </div>
              <div className="hidden leading-none sm:block">
                <span className="block text-sm font-semibold">你济社区</span>
                <span className="block text-[11px] text-muted-foreground">YourTJ</span>
              </div>
            </Link>

            <button
              type="button"
              onClick={() => setSearchOpen(true)}
              className="ml-1 hidden h-9 max-w-xl flex-1 items-center gap-2 rounded-md border bg-background px-3 text-left text-sm text-muted-foreground transition-colors hover:bg-accent md:flex"
            >
              <Search className="h-4 w-4" />
              搜索课程、帖子、点评
              <span className="ml-auto rounded border px-1.5 py-0.5 text-[10px]">/</span>
            </button>

            <div className="ml-auto flex items-center gap-1">
              <Button variant="ghost" size="icon" className="md:hidden" onClick={() => setSearchOpen(true)} aria-label="搜索">
                <Search className="h-4 w-4" />
              </Button>
              <ThemeToggle />
              {isAuthenticated ? (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button className="rounded-full focus-visible:ring-[3px] focus-visible:ring-ring/50">
                      <Avatar>
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
                      <User className="h-4 w-4" />
                      个人主页
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => navigate("/settings")}>
                      <Settings className="h-4 w-4" />
                      设置
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onSelect={() => void logout()}>
                      <LogOut className="h-4 w-4" />
                      退出登录
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : (
                <Button asChild size="sm">
                  <Link to="/login">登录</Link>
                </Button>
              )}
            </div>
          </div>
        </header>

        <div className="mx-auto grid max-w-7xl grid-cols-1 gap-6 px-4 py-6 lg:grid-cols-[13rem_minmax(0,1fr)]">
          <aside className="hidden lg:block">
            <div className="sticky top-20">
              <NavItems />
            </div>
          </aside>
          <main className="min-w-0 pb-16">
            <Outlet />
          </main>
        </div>
      </div>
      <SearchDialog open={searchOpen} onOpenChange={setSearchOpen} />
    </TooltipProvider>
  );
}
