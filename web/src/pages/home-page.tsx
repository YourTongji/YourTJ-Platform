import { useQuery } from "@tanstack/react-query";
import * as React from "react";

import { ActivityHeatmap } from "@/components/activity/activity-heatmap";
import { getTwentyWeekActivityRange } from "@/components/activity/calendar-range";
import { CommunityFeed, type CommunityFeedMode } from "@/components/home/community-feed";
import { CommunitySidebar } from "@/components/home/community-sidebar";
import { Card, CardContent } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";

export function HomePage() {
  const [feedMode, setFeedMode] = React.useState<CommunityFeedMode>("hot");
  const { account } = useAuth();
  const activityRange = React.useMemo(() => getTwentyWeekActivityRange(), []);
  const threads = useQuery({
    queryKey: ["home", "threads", feedMode],
    queryFn: () => api.threads({ feed: feedMode }),
  });
  const announcements = useQuery({
    queryKey: ["announcements", "active"],
    queryFn: api.announcements,
  });
  const activity = useQuery({
    queryKey: ["home", "activity", account?.id, activityRange.from, activityRange.to],
    queryFn: () => api.myActivity(activityRange.from, activityRange.to),
    enabled: Boolean(account),
  });

  const threadItems = threads.data?.items ?? [];

  return (
    <div className="min-[1240px]:grid min-[1240px]:grid-cols-[minmax(0,640px)_320px]">
      <div className="px-4 py-5 sm:px-6 sm:py-6 min-[1360px]:!px-8">
        <Card className="mb-5 rounded-xl min-[1240px]:hidden">
          <CardContent className="p-4">
            <ActivityHeatmap
              isAuthenticated={Boolean(account)}
              calendar={activity.data}
              isLoading={activity.isLoading}
              error={activity.error}
              onRetry={() => void activity.refetch()}
            />
          </CardContent>
        </Card>
        <CommunityFeed
          mode={feedMode}
          onModeChange={setFeedMode}
          items={threadItems}
          isLoading={threads.isLoading}
          error={threads.error}
          onRetry={() => void threads.refetch()}
        />
      </div>

      <div className="hidden pb-16 pl-6 pt-6 min-[1240px]:block">
        <CommunitySidebar
          account={account}
          activity={{
            calendar: activity.data,
            isLoading: activity.isLoading,
            error: activity.error,
            onRetry: () => void activity.refetch(),
          }}
          threads={threadItems}
          announcements={announcements.data ?? []}
        />
      </div>
    </div>
  );
}
