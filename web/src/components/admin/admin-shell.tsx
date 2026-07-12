import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export interface AdminNavigationItem {
  id: string;
  label: string;
  description: string;
  icon: LucideIcon;
}

export function AdminShell({
  items,
  active,
  onActiveChange,
  children,
}: {
  items: AdminNavigationItem[];
  active: string;
  onActiveChange: (id: string) => void;
  children: ReactNode;
}) {
  return (
    <div className="grid min-w-0 gap-5 lg:grid-cols-[13rem_minmax(0,1fr)]">
      <Card className="h-fit min-w-0 max-w-full overflow-hidden rounded-xl lg:sticky lg:top-20">
        <CardContent className="scrollbar-none flex w-full min-w-0 gap-1 overflow-x-auto p-2 lg:flex-col" role="navigation" aria-label="管理后台功能">
          {items.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-current={active === item.id ? "page" : undefined}
              onClick={() => onActiveChange(item.id)}
              className={cn(
                "flex min-w-fit items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors lg:min-w-0",
                active === item.id
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
            >
              <item.icon className="size-4 shrink-0" aria-hidden="true" />
              <span className="min-w-0">
                <span className="block text-sm font-medium">{item.label}</span>
                <span className="hidden truncate text-[10px] opacity-75 lg:block">{item.description}</span>
              </span>
            </button>
          ))}
        </CardContent>
      </Card>
      <section className="min-w-0" aria-live="polite">{children}</section>
    </div>
  );
}
