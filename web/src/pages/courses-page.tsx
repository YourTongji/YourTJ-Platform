import { useQuery } from "@tanstack/react-query";
import { BookOpen, Search } from "lucide-react";
import * as React from "react";
import { Link, useSearchParams } from "react-router";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { PageHeader } from "@/components/common/page-header";
import { RatingStars } from "@/components/common/rating-stars";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api } from "@/lib/api/endpoints";
import type { Course } from "@/lib/api/types";
import { formatNumber, formatRating } from "@/lib/format";

function CourseCard({ course }: { course: Course }) {
  return (
    <Link to={`/courses/${course.id}`} className="block">
      <Card className="h-full transition-shadow hover:shadow-md">
        <CardContent className="p-4">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h2 className="line-clamp-2 font-semibold">{course.name}</h2>
              <p className="mt-1 text-sm text-muted-foreground">
                {course.code} · {course.teacherName ?? "教师待同步"}
              </p>
            </div>
            <Badge variant="secondary">{course.credit ?? 0} 学分</Badge>
          </div>
          <div className="mt-4 flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <RatingStars value={course.reviewAvg ?? 0} />
                <span className="text-sm font-medium">{formatRating(course.reviewAvg)}</span>
              </div>
              <p className="mt-1 text-xs text-muted-foreground">{formatNumber(course.reviewCount)} 条点评</p>
            </div>
            <BookOpen className="h-5 w-5 text-primary" />
          </div>
          {course.department ? (
            <p className="mt-3 line-clamp-1 text-xs text-muted-foreground">{course.department}</p>
          ) : null}
        </CardContent>
      </Card>
    </Link>
  );
}

export function CoursesPage() {
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = React.useState(params.get("q") ?? "");
  const dept = params.get("dept") ?? "all";
  const sort = (params.get("sort") as "hot" | "rating" | "new" | null) ?? "hot";
  const q = params.get("q") ?? "";

  const departments = useQuery({ queryKey: ["departments"], queryFn: api.departments });
  const courses = useQuery({
    queryKey: ["courses", dept, sort],
    queryFn: () => api.courses({ dept: dept === "all" ? undefined : dept, sort }),
    enabled: q.length < 2,
  });
  const search = useQuery({
    queryKey: ["course-search", q],
    queryFn: () => api.search(q, "course"),
    enabled: q.length >= 2,
  });

  const data: Course[] = q.length >= 2 ? search.data?.courses ?? [] : courses.data?.items ?? [];
  const isLoading = q.length >= 2 ? search.isLoading : courses.isLoading;
  const error = q.length >= 2 ? search.error : courses.error;
  const isError = q.length >= 2 ? search.isError : courses.isError;

  function update(next: Record<string, string | null>) {
    const copy = new URLSearchParams(params);
    for (const [key, value] of Object.entries(next)) {
      if (!value || value === "all") {
        copy.delete(key);
      } else {
        copy.set(key, value);
      }
    }
    setParams(copy);
  }

  return (
    <div>
      <PageHeader
        eyebrow="Courses"
        title="课程点评"
        description="浏览课程、查看点评统计和 AI 摘要；实时搜索由后端 Meilisearch 支撑。"
      />

      <div className="mb-5 grid gap-3 lg:grid-cols-[1fr_14rem_18rem]">
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                update({ q: query.trim() || null });
              }
            }}
            placeholder="搜索课程名、课号、教师、拼音"
            className="pl-9"
          />
        </div>
        <Button variant="outline" onClick={() => update({ q: query.trim() || null })}>
          搜索
        </Button>
        <Select value={dept} onValueChange={(value) => update({ dept: value })}>
          <SelectTrigger>
            <SelectValue placeholder="院系" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部院系</SelectItem>
            {(departments.data ?? []).map((item) => (
              <SelectItem key={item.id} value={item.id ?? item.name ?? ""}>
                {item.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <Tabs value={sort} onValueChange={(value) => update({ sort: value })} className="mb-5">
        <TabsList>
          <TabsTrigger value="hot">热门</TabsTrigger>
          <TabsTrigger value="rating">评分</TabsTrigger>
          <TabsTrigger value="new">最新</TabsTrigger>
        </TabsList>
      </Tabs>

      {isLoading ? (
        <LoadingState />
      ) : isError ? (
        <ErrorState error={error} onRetry={() => (q.length >= 2 ? void search.refetch() : void courses.refetch())} />
      ) : data.length === 0 ? (
        <EmptyState title="没有找到课程" description="换一个关键词或院系再试。" />
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {data.map((course) => (
            <CourseCard key={course.id ?? course.code} course={course} />
          ))}
        </div>
      )}
    </div>
  );
}
