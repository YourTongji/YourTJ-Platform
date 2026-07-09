import { useQuery } from "@tanstack/react-query";
import { MessageSquare, User } from "lucide-react";
import { Link, useParams } from "react-router";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { TeaBadge } from "@/components/common/tea-badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { api } from "@/lib/api/endpoints";
import { formatDate, formatUnixTime } from "@/lib/format";

export function ProfilePage() {
  const { handle } = useParams();
  const name = handle ?? "";
  const profile = useQuery({ queryKey: ["profile", name], queryFn: () => api.publicUser(name), enabled: Boolean(name) });
  const threads = useQuery({ queryKey: ["profile", name, "threads"], queryFn: () => api.userThreads(name), enabled: Boolean(name) });
  const comments = useQuery({ queryKey: ["profile", name, "comments"], queryFn: () => api.userComments(name), enabled: Boolean(name) });

  if (profile.isLoading) {
    return <LoadingState label="加载用户主页" />;
  }
  if (profile.isError || !profile.data) {
    return <ErrorState error={profile.error} onRetry={() => void profile.refetch()} />;
  }

  return (
    <div className="space-y-5">
      <PageHeader eyebrow="Profile" title={profile.data.handle ?? name} description={`加入于 ${formatDate(profile.data.createdAt)}`} />
      <Card>
        <CardContent className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center">
          <Avatar className="h-16 w-16">
            <AvatarImage src={profile.data.avatarUrl ?? undefined} />
            <AvatarFallback>{profile.data.handle?.slice(0, 1).toUpperCase()}</AvatarFallback>
          </Avatar>
          <div className="flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-xl font-semibold">{profile.data.handle}</h2>
              <TeaBadge level={profile.data.trustLevel ?? 0} />
              <Badge variant="secondary">{profile.data.role ?? "user"}</Badge>
            </div>
            <div className="mt-2 flex flex-wrap gap-3 text-sm text-muted-foreground">
              <span>{profile.data.threadCount ?? 0} 帖子</span>
              <span>{profile.data.commentCount ?? 0} 回复</span>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-5 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><MessageSquare className="h-4 w-4 text-primary" />主题</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {(threads.data?.items ?? []).length === 0 ? (
              <EmptyState title="暂无主题" />
            ) : (
              threads.data?.items?.map((thread) => (
                <Link key={thread.id} to={`/forum/threads/${thread.id}`} className="block rounded-md border p-3 hover:bg-accent">
                  <p className="font-medium">{thread.title}</p>
                  <p className="text-xs text-muted-foreground">{formatUnixTime(thread.createdAt)}</p>
                </Link>
              ))
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><User className="h-4 w-4 text-primary" />回复</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {(comments.data?.items ?? []).length === 0 ? (
              <EmptyState title="暂无回复" />
            ) : (
              comments.data?.items?.map((comment) => (
                <Link key={comment.id} to={`/forum/threads/${comment.threadId}`} className="block rounded-md border p-3 hover:bg-accent">
                  <p className="line-clamp-2 text-sm">{comment.body}</p>
                  <p className="text-xs text-muted-foreground">{formatUnixTime(comment.createdAt)}</p>
                </Link>
              ))
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
