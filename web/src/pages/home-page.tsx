import { useQuery } from "@tanstack/react-query";
import * as React from "react";

import { CommunityFeed, type CommunityFeedMode } from "@/components/home/community-feed";
import { CommunitySidebar } from "@/components/home/community-sidebar";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";

export function HomePage() {
  const [feedMode, setFeedMode] = React.useState<CommunityFeedMode>("hot");
  const { account } = useAuth();
  const threads = useQuery({
    queryKey: ["home", "threads", feedMode],
    queryFn: () => api.threads({ feed: feedMode }),
  });
  const announcements = useQuery({
    queryKey: ["home", "announcements"],
    queryFn: api.announcements,
  });

  const threadItems = threads.data?.items ?? [];

  return (
    <div className="min-[1240px]:grid min-[1240px]:grid-cols-[minmax(0,640px)_320px]">
      <div className="px-4 py-5 sm:px-6 sm:py-6 min-[1360px]:!px-8">
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
          threads={threadItems}
          announcements={announcements.data ?? []}
        />
      </div>
    </div>
  );
}
