import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  CalendarDays,
  FileJson2,
  FileUp,
  ImageDown,
  Plus,
  Search,
  Trash2,
} from "lucide-react";
import * as React from "react";
import { useSearchParams } from "react-router";
import { toast } from "sonner";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { PageHeader } from "@/components/common/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAuth } from "@/context/auth-provider";
import { API_BASE_URL } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import type { SelectionCourse, TimeSlot } from "@/lib/api/types";
import { formatDate } from "@/lib/format";
import {
  findScheduleConflicts,
  offeringKey,
  parseScheduleImport,
  serializeScheduleExport,
  type ScheduleConflict,
  type ScheduledCourse,
  type ScheduleScope,
  timetableCells,
  timetableMaxSlot,
  useScheduleStore,
} from "@/lib/schedule-store";
import { cn } from "@/lib/utils";

const weekdays = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
const metadataStaleTime = 30 * 60 * 1000;

function useDebouncedValue<T>(value: T, delay: number) {
  const [debounced, setDebounced] = React.useState(value);
  React.useEffect(() => {
    const timeout = window.setTimeout(() => setDebounced(value), delay);
    return () => window.clearTimeout(timeout);
  }, [delay, value]);
  return debounced;
}

function teachers(course: SelectionCourse) {
  const names = course.teacherNames.length
    ? course.teacherNames
    : course.teacherName
      ? [course.teacherName]
      : [];
  return names.length ? names.join(" / ") : "教师待同步";
}

function weekLabel(course: SelectionCourse) {
  if (course.weeksUnknown || course.startWeek === null || course.endWeek === null) {
    return "周次待确认";
  }
  return `${course.startWeek}-${course.endWeek} 周`;
}

function reviewLabel(course: SelectionCourse) {
  if (course.reviewCount === 0 || course.reviewAvg === null) return "暂无历史评分";
  const scope = course.reviewScope === "teacher" ? "当前教师" : "课程参考";
  return `${course.reviewAvg.toFixed(1)} 分 · ${course.reviewCount} 条历史评课 · ${scope}`;
}

function slotLabel(slot: TimeSlot) {
  const week = slot.weeksUnknown
    ? "周次待确认"
    : slot.weekNumbers.length > 0
      ? `${slot.weekNumbers.join(",")} 周`
      : slot.weeks ?? "周次待确认";
  return `${weekdays[slot.weekday - 1] ?? `周${slot.weekday}`} ${slot.startSlot}-${slot.endSlot} 节 · ${slot.location ?? "地点待定"} · ${week}`;
}

function CourseRow({
  course,
  staged,
  pending,
  highlighted,
  onAdd,
}: {
  course: SelectionCourse;
  staged: boolean;
  pending: boolean;
  highlighted: boolean;
  onAdd: (course: SelectionCourse) => void;
}) {
  return (
    <article
      className={cn(
        "rounded-xl border bg-card p-4 shadow-sm transition-colors",
        highlighted && "border-primary ring-2 ring-primary/20",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-medium">{course.name}</p>
            <Badge variant="secondary">{course.credit ?? 0} 学分</Badge>
            {course.scheduleUnknown ? <Badge variant="destructive">时段未知</Badge> : null}
            {!course.scheduleUnknown && course.weeksUnknown ? (
              <Badge variant="outline">周次未知</Badge>
            ) : null}
          </div>
          <p className="mt-1 text-sm text-muted-foreground">
            {course.code} · 教学班 {course.teachingClassCode ?? course.offeringId}
          </p>
          <p className="mt-1 text-sm text-muted-foreground">
            {teachers(course)} · {weekLabel(course)}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">{reviewLabel(course)}</p>
        </div>
        <Button
          size="sm"
          variant={staged ? "secondary" : "default"}
          onClick={() => onAdd(course)}
          disabled={staged || pending}
          aria-label={`${staged ? "已加入" : pending ? "正在读取" : "加入"}${course.name}教学班${course.teachingClassCode ?? course.offeringId}`}
        >
          <Plus className="h-4 w-4" />
          {staged ? "已加入" : pending ? "读取中" : "加入"}
        </Button>
      </div>
    </article>
  );
}

function conflictMap(staged: ScheduledCourse[]) {
  const result = new Map<string, ScheduleConflict["certainty"]>();
  for (let index = 0; index < staged.length; index += 1) {
    for (let next = index + 1; next < staged.length; next += 1) {
      const left = staged[index];
      const right = staged[next];
      if (!left || !right) continue;
      const conflict = findScheduleConflicts([left], right.course, right.timeslots)[0];
      if (!conflict) continue;
      const certainty = conflict.certainty;
      for (const id of [offeringKey(left.course), offeringKey(right.course)]) {
        if (certainty === "confirmed" || !result.has(id)) result.set(id, certainty);
      }
    }
  }
  return result;
}

function Timetable({ staged, onSelect }: { staged: ScheduledCourse[]; onSelect: (item: ScheduledCourse) => void }) {
  const maxSlot = timetableMaxSlot(staged);
  const cells = timetableCells(staged, maxSlot);
  const conflicts = conflictMap(staged);
  return (
    <div className="overflow-x-auto rounded-xl border bg-card" role="region" aria-label="每周课表">
      <div className="grid min-w-[840px] grid-cols-[4rem_repeat(7,minmax(6.5rem,1fr))]">
        <div className="sticky left-0 top-0 z-20 border-b border-r bg-muted p-2 text-center text-xs font-medium text-muted-foreground">
          节次
        </div>
        {weekdays.map((weekday) => (
          <div key={weekday} className="sticky top-0 z-10 border-b border-r bg-muted p-2 text-center text-xs font-medium">
            {weekday}
          </div>
        ))}
        {Array.from({ length: maxSlot }, (_, index) => index + 1).map((section) => (
          <React.Fragment key={section}>
            <div className="sticky left-0 z-10 border-b border-r bg-card p-2 text-center text-xs text-muted-foreground">
              {section}
            </div>
            {weekdays.map((_, weekdayIndex) => {
              const key = `${weekdayIndex + 1}-${section}`;
              return (
                <div key={key} className="min-h-16 border-b border-r p-1.5">
                  <div className="space-y-1">
                    {(cells[key] ?? []).map((item) => {
                      const conflict = conflicts.get(offeringKey(item.course));
                      return (
                        <button
                          type="button"
                          key={`${offeringKey(item.course)}-${section}`}
                          className={cn(
                            "w-full rounded border border-white/60 px-2 py-1 text-left text-xs leading-snug text-white shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                            conflict && "ring-2 ring-destructive ring-offset-1",
                          )}
                          style={{ backgroundColor: item.color }}
                          onClick={() => onSelect(item)}
                          aria-label={`${item.course.name}，${conflict === "confirmed" ? "已确认冲突" : conflict === "possible" ? "可能冲突" : "无已知冲突"}`}
                        >
                          <span className="line-clamp-2 font-medium">{item.course.name}</span>
                          <span className="block opacity-90">{item.course.teachingClassCode ?? item.course.offeringId}</span>
                          {conflict ? (
                            <span className="block font-semibold">
                              {conflict === "confirmed" ? "冲突" : "可能冲突"}
                            </span>
                          ) : null}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

function MobileDaySchedule({
  staged,
  onSelect,
}: {
  staged: ScheduledCourse[];
  onSelect: (item: ScheduledCourse) => void;
}) {
  const [weekday, setWeekday] = React.useState("1");
  const day = Number(weekday);
  const rows = staged
    .flatMap((item) => item.timeslots.map((slot) => ({ item, slot })))
    .filter(({ slot }) => slot.weekday === day)
    .sort((left, right) => left.slot.startSlot - right.slot.startSlot);
  const conflicts = conflictMap(staged);
  return (
    <div className="space-y-3 md:hidden">
      <div className="flex w-full gap-1 overflow-x-auto rounded-lg bg-muted p-1" aria-label="选择课表日期">
        {weekdays.map((label, index) => {
          const value = String(index + 1);
          return (
            <Button
              key={label}
              type="button"
              size="sm"
              variant={weekday === value ? "secondary" : "ghost"}
              onClick={() => setWeekday(value)}
              aria-pressed={weekday === value}
            >
              {label}
            </Button>
          );
        })}
      </div>
      {rows.length === 0 ? (
        <EmptyState title={`${weekdays[day - 1]}暂无课程`} description="切换日期或从课程结果中加入教学班。" />
      ) : (
        rows.map(({ item, slot }) => {
          const conflict = conflicts.get(offeringKey(item.course));
          return (
            <button
              type="button"
              key={`${offeringKey(item.course)}-${slot.startSlot}-${slot.endSlot}`}
              className={cn(
                "w-full rounded-xl border bg-card p-4 text-left shadow-sm",
                conflict && "border-destructive",
              )}
              onClick={() => onSelect(item)}
            >
              <div className="flex items-start gap-3">
                <span className="mt-1 h-10 w-1 shrink-0 rounded-full" style={{ backgroundColor: item.color }} />
                <span className="min-w-0">
                  <span className="font-medium">{item.course.name}</span>
                  <span className="mt-1 block text-sm text-muted-foreground">{slotLabel(slot)}</span>
                  {conflict ? (
                    <Badge className="mt-2" variant="destructive">
                      {conflict === "confirmed" ? "已确认冲突" : "可能冲突"}
                    </Badge>
                  ) : null}
                </span>
              </div>
            </button>
          );
        })
      )}
    </div>
  );
}

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

function exportJson(staged: ScheduledCourse[], scope: ScheduleScope) {
  downloadBlob(
    new Blob([serializeScheduleExport(staged, scope)], { type: "application/json;charset=utf-8" }),
    `yourtj-schedule-${scope.calendarId}.json`,
  );
}

async function exportPng(staged: ScheduledCourse[], calendarId: string) {
  const maxSlot = timetableMaxSlot(staged);
  const width = 1400;
  const rowHeight = 72;
  const incompleteCount = staged.filter((item) => (
    item.course.scheduleUnknown
    || item.course.weeksUnknown
    || item.timeslots.some((slot) => slot.weeksUnknown)
  )).length;
  const noticeHeight = incompleteCount > 0 ? 36 : 0;
  const headerHeight = 64 + noticeHeight;
  const height = headerHeight + maxSlot * rowHeight;
  const canvas = document.createElement("canvas");
  const scale = Math.min(window.devicePixelRatio || 1, 2);
  canvas.width = width * scale;
  canvas.height = height * scale;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("浏览器不支持课表图片导出");
  context.scale(scale, scale);
  context.fillStyle = "#fbfbf7";
  context.fillRect(0, 0, width, height);
  const labelWidth = 72;
  const dayWidth = (width - labelWidth) / 7;
  context.strokeStyle = "#d9ddd7";
  context.fillStyle = "#26332f";
  context.font = "600 16px system-ui, sans-serif";
  if (incompleteCount > 0) {
    context.fillStyle = "#8f321f";
    context.fillText(`注意：${incompleteCount} 个教学班的时段或周次不完整`, 20, 24);
    context.fillStyle = "#26332f";
  }
  weekdays.forEach((label, index) => {
    context.fillText(label, labelWidth + index * dayWidth + 18, noticeHeight + 38);
  });
  context.font = "13px system-ui, sans-serif";
  for (let section = 1; section <= maxSlot; section += 1) {
    const y = headerHeight + (section - 1) * rowHeight;
    context.fillStyle = "#5f6f69";
    context.fillText(String(section), 30, y + 38);
    context.beginPath();
    context.moveTo(0, y);
    context.lineTo(width, y);
    context.stroke();
  }
  for (let day = 0; day <= 7; day += 1) {
    const x = labelWidth + day * dayWidth;
    context.beginPath();
    context.moveTo(x, 0);
    context.lineTo(x, height);
    context.stroke();
  }
  for (const item of staged) {
    for (const slot of item.timeslots) {
      const x = labelWidth + (slot.weekday - 1) * dayWidth + 5;
      const y = headerHeight + (slot.startSlot - 1) * rowHeight + 5;
      const blockHeight = (slot.endSlot - slot.startSlot + 1) * rowHeight - 10;
      context.fillStyle = item.color;
      context.fillRect(x, y, dayWidth - 10, blockHeight);
      context.fillStyle = "#ffffff";
      context.font = "600 14px system-ui, sans-serif";
      context.fillText(item.course.name.slice(0, 14), x + 8, y + 24, dayWidth - 26);
      context.font = "12px system-ui, sans-serif";
      context.fillText(
        (item.course.teachingClassCode ?? item.course.offeringId).slice(0, 18),
        x + 8,
        y + 44,
        dayWidth - 26,
      );
    }
  }
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((value) => value ? resolve(value) : reject(new Error("课表图片生成失败")), "image/png");
  });
  downloadBlob(blob, `yourtj-schedule-${calendarId}.png`);
}

export function SchedulePage() {
  const { account } = useAuth();
  const [searchParams] = useSearchParams();
  const deepLinkCode = searchParams.get("courseCode")?.trim() ?? "";
  const [calendarId, setCalendarId] = React.useState("");
  const [grade, setGrade] = React.useState("");
  const [majorId, setMajorId] = React.useState("");
  const [natureId, setNatureId] = React.useState("");
  const [searchText, setSearchText] = React.useState(deepLinkCode);
  const [activeTab, setActiveTab] = React.useState(deepLinkCode ? "search" : "major");
  const [weekday, setWeekday] = React.useState("any");
  const [startSlot, setStartSlot] = React.useState("any");
  const [endSlot, setEndSlot] = React.useState("any");
  const [week, setWeek] = React.useState("any");
  const [mobileView, setMobileView] = React.useState("browse");
  const [clearOpen, setClearOpen] = React.useState(false);
  const [pendingAdd, setPendingAdd] = React.useState<{
    course: SelectionCourse;
    timeslots: TimeSlot[];
    conflicts: ScheduleConflict[];
  } | null>(null);
  const [selectedCourse, setSelectedCourse] = React.useState<ScheduledCourse | null>(null);
  const importInput = React.useRef<HTMLInputElement | null>(null);
  const debouncedSearch = useDebouncedValue(searchText.trim(), 350);

  const calendars = useQuery({
    queryKey: ["selection", "calendars"],
    queryFn: api.calendars,
    staleTime: metadataStaleTime,
  });
  const latest = useQuery({
    queryKey: ["selection", "latest"],
    queryFn: api.selectionLatestUpdate,
    staleTime: 5 * 60 * 1000,
  });
  const grades = useQuery({
    queryKey: ["selection", "grades", calendarId],
    queryFn: () => api.grades(calendarId),
    enabled: Boolean(calendarId),
    staleTime: metadataStaleTime,
  });
  const majors = useQuery({
    queryKey: ["selection", "majors", calendarId, grade],
    queryFn: () => api.majors(calendarId, grade),
    enabled: Boolean(calendarId && grade),
    staleTime: metadataStaleTime,
  });
  const natures = useQuery({
    queryKey: ["selection", "natures", calendarId],
    queryFn: () => api.courseNatures(calendarId),
    enabled: Boolean(calendarId),
    staleTime: metadataStaleTime,
  });

  React.useEffect(() => {
    const current = calendars.data?.find((calendar) => calendar.isCurrent);
    if (!calendarId && current?.id) setCalendarId(current.id);
  }, [calendarId, calendars.data]);

  React.useEffect(() => {
    if (!deepLinkCode) return;
    setSearchText(deepLinkCode);
    setActiveTab("search");
  }, [deepLinkCode]);

  const timeFilterTouched = [weekday, startSlot, endSlot, week].some((value) => value !== "any");
  const timeComplete = weekday !== "any" && startSlot !== "any" && endSlot !== "any";
  const timeRangeValid = !timeComplete || Number(startSlot) <= Number(endSlot);
  const timeFiltersValid = !timeFilterTouched || (timeComplete && timeRangeValid);
  const selectionReady = Boolean(calendarId) && (
    (activeTab === "major" && majorId && grade)
    || (activeTab === "nature" && natureId)
    || (activeTab === "search" && debouncedSearch.length >= 2)
  );
  const browseEnabled = Boolean(selectionReady && timeFiltersValid);
  const offerings = useInfiniteQuery({
    queryKey: [
      "selection",
      "offerings",
      calendarId,
      activeTab,
      majorId,
      grade,
      natureId,
      debouncedSearch,
      weekday,
      startSlot,
      endSlot,
      week,
    ],
    queryFn: ({ pageParam, signal }) => api.selectionOfferings({
      calendarId,
      majorId: activeTab === "major" ? majorId : undefined,
      grade: activeTab === "major" ? grade : undefined,
      natureId: activeTab === "nature" ? natureId : undefined,
      q: activeTab === "search" ? debouncedSearch : undefined,
      weekday: timeComplete ? Number(weekday) : undefined,
      startSlot: timeComplete ? Number(startSlot) : undefined,
      endSlot: timeComplete ? Number(endSlot) : undefined,
      week: timeComplete && week !== "any" ? Number(week) : undefined,
      includeUnknownSchedule: true,
      cursor: pageParam,
      limit: 20,
    }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: Boolean(browseEnabled),
    staleTime: 60 * 1000,
  });

  const scope = React.useMemo<ScheduleScope>(
    () => ({
      environment: API_BASE_URL,
      principal: account?.id ?? "anonymous",
      calendarId,
    }),
    [account?.id, calendarId],
  );
  const staged = useScheduleStore(scope, (state) => state.staged);
  const addCourse = useScheduleStore(scope, (state) => state.addCourse);
  const removeCourse = useScheduleStore(scope, (state) => state.removeCourse);
  const restoreSchedule = useScheduleStore(scope, (state) => state.restore);
  const clearSchedule = useScheduleStore(scope, (state) => state.clear);
  const stagedIds = React.useMemo(() => new Set(staged.map((item) => offeringKey(item.course))), [staged]);
  const offeringItems = offerings.data?.pages.flatMap((page) => page.items) ?? [];
  const addRequest = React.useRef<AbortController | null>(null);
  const activeScope = React.useRef(scope);
  activeScope.current = scope;
  const [pendingOfferingId, setPendingOfferingId] = React.useState<string | null>(null);

  React.useEffect(() => {
    addRequest.current?.abort();
    addRequest.current = null;
    setPendingOfferingId(null);
    setPendingAdd(null);
  }, [scope]);

  React.useEffect(() => () => addRequest.current?.abort(), []);

  async function handleAddCourse(course: SelectionCourse) {
    if (stagedIds.has(offeringKey(course))) {
      toast.info("这个教学班已经在待选课表中");
      return;
    }
    addRequest.current?.abort();
    const controller = new AbortController();
    addRequest.current = controller;
    setPendingOfferingId(course.offeringId);
    const requestScope = scope;
    try {
      const timeslots = await api.selectionOfferingTimeslots(course.offeringId, controller.signal);
      if (controller.signal.aborted || activeScope.current !== requestScope) return;
      const result = addCourse(course, timeslots);
      if (result.status === "duplicate") {
        toast.info("这个教学班已经在待选课表中");
      } else if (result.status === "scopeMismatch") {
        toast.error("教学班不属于当前学期，请刷新后重试");
      } else if (result.status === "conflict") {
        setPendingAdd({ course, timeslots, conflicts: result.conflicts });
      } else if (result.status === "added") {
        if (course.scheduleUnknown || timeslots.some((slot) => slot.weeksUnknown)) {
          toast.warning(`${course.name} 已加入，但上游时段或周次仍不完整`);
        } else {
          toast.success(`${course.name} 已加入课表`);
        }
      }
    } catch (error) {
      if (controller.signal.aborted) return;
      toast.error(error instanceof Error ? error.message : "拉取教学班时段失败");
    } finally {
      if (addRequest.current === controller) {
        addRequest.current = null;
        setPendingOfferingId(null);
      }
    }
  }

  function confirmConflictingAdd() {
    if (!pendingAdd) return;
    const result = addCourse(pendingAdd.course, pendingAdd.timeslots, {
      allowPossibleConflict: true,
    });
    setPendingAdd(null);
    if (result.status === "added") toast.warning(`${pendingAdd.course.name} 已带冲突标记加入`);
    if (result.status === "duplicate") toast.info("这个教学班已经在待选课表中");
    if (result.status === "scopeMismatch") toast.error("教学班不属于当前学期，请刷新后重试");
    if (result.status === "conflict") toast.error("课表已有变化，当前教学班存在确定冲突，未加入");
  }

  async function importJson(file: File) {
    try {
      if (file.size > 2 * 1024 * 1024) throw new Error("课表 JSON 不能超过 2 MB");
      const restored = parseScheduleImport(await file.text(), scope);
      restoreSchedule(restored);
      setPendingAdd(null);
      setSelectedCourse(null);
      toast.success(`已恢复 ${restored.length} 个教学班`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "课表文件导入失败");
    } finally {
      if (importInput.current) importInput.current.value = "";
    }
  }

  function handleCalendarChange(nextCalendarId: string) {
    setCalendarId(nextCalendarId);
    setGrade("");
    setMajorId("");
    setNatureId("");
    setSearchText("");
    setPendingAdd(null);
    setSelectedCourse(null);
  }

  const totalCredits = staged.reduce((sum, item) => sum + Number(item.course.credit ?? 0), 0);
  const pendingHasConfirmedConflict = pendingAdd?.conflicts.some(
    ({ certainty }) => certainty === "confirmed",
  ) ?? false;

  if (calendars.isLoading) return <LoadingState label="加载选课基础数据" />;
  if (calendars.isError) {
    return <ErrorState error={calendars.error} onRetry={() => void calendars.refetch()} />;
  }

  const resultsContent = selectionReady && !timeFiltersValid ? (
    <EmptyState
      title="修正时间筛选"
      description={timeComplete ? "起始节不能晚于结束节。" : "星期、起始节和结束节必须同时选择。"}
    />
  ) : !selectionReady ? (
    <EmptyState
      title={activeTab === "major" ? "先选择年级和专业" : activeTab === "nature" ? "选择课程性质" : "输入至少两个字符"}
      description="结果按教学班展示，同一课程的不同教师或时段不会再被合并。"
    />
  ) : offerings.isLoading ? (
    <LoadingState label="加载教学班" />
  ) : offerings.isError ? (
    <ErrorState error={offerings.error} onRetry={() => void offerings.refetch()} />
  ) : offeringItems.length === 0 ? (
    <EmptyState title="没有找到教学班" description="调整筛选条件或保留“未知时段”后再试。" />
  ) : (
    <div className="space-y-3">
      {offeringItems.map((course) => (
        <CourseRow
          key={course.offeringId}
          course={course}
          staged={stagedIds.has(offeringKey(course))}
          pending={pendingOfferingId === course.offeringId}
          highlighted={Boolean(deepLinkCode && course.code === deepLinkCode)}
          onAdd={(item) => void handleAddCourse(item)}
        />
      ))}
      {offerings.hasNextPage ? (
        <div className="flex justify-center">
          <Button
            variant="outline"
            onClick={() => void offerings.fetchNextPage()}
            disabled={offerings.isFetchingNextPage}
          >
            {offerings.isFetchingNextPage ? "加载中…" : "加载更多教学班"}
          </Button>
        </div>
      ) : null}
    </div>
  );

  return (
    <div>
      <PageHeader
        title="选课排课"
        description="按教学班浏览、检查周次冲突，并将待选课表仅保存在当前环境与账号的本机空间。"
        actions={
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              onClick={() => {
                try {
                  exportJson(staged, scope);
                } catch (error) {
                  toast.error(error instanceof Error ? error.message : "导出失败");
                }
              }}
              disabled={staged.length === 0 || !calendarId}
            >
              <FileJson2 className="h-4 w-4" />导出 JSON
            </Button>
            <Button
              variant="outline"
              onClick={() => importInput.current?.click()}
              disabled={!calendarId}
            >
              <FileUp className="h-4 w-4" />导入 JSON
            </Button>
            <input
              ref={importInput}
              type="file"
              accept="application/json,.json"
              className="sr-only"
              aria-label="导入课表 JSON"
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) void importJson(file);
              }}
            />
            <Button
              variant="outline"
              onClick={() => void exportPng(staged, calendarId).catch((error) => toast.error(error instanceof Error ? error.message : "导出失败"))}
              disabled={staged.length === 0}
            >
              <ImageDown className="h-4 w-4" />导出 PNG
            </Button>
            <Button variant="outline" onClick={() => setClearOpen(true)} disabled={staged.length === 0}>
              <Trash2 className="h-4 w-4" />清空
            </Button>
          </div>
        }
      />

      <div className="mb-4 grid w-full grid-cols-3 gap-1 rounded-lg bg-muted p-1 xl:hidden" aria-label="排课页面视图">
        {[
          ["browse", "找课"],
          ["schedule", "课表"],
          ["selected", `待选 ${staged.length}`],
        ].map(([value, label]) => (
          <Button
            key={value}
            type="button"
            size="sm"
            variant={mobileView === value ? "secondary" : "ghost"}
            onClick={() => setMobileView(value ?? "browse")}
            aria-pressed={mobileView === value}
          >
            {label}
          </Button>
        ))}
      </div>

      <div className="grid gap-5 xl:grid-cols-[18rem_minmax(0,1fr)_21rem]">
        <aside aria-label="选课筛选" className={cn("space-y-4", mobileView !== "browse" && "hidden xl:block")}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <CalendarDays className="h-5 w-5 text-primary" />选课上下文
              </CardTitle>
              <CardDescription>
                上游更新：{formatDate(latest.data?.updatedAt ?? null)}
              </CardDescription>
              {latest.data?.stale ? (
                <Badge variant="destructive" className="w-fit">数据已超过 {latest.data.staleAfterHours} 小时</Badge>
              ) : null}
              {latest.data?.importedAt ? (
                <p className="text-xs text-muted-foreground">快照导入：{formatDate(latest.data.importedAt)}</p>
              ) : null}
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-2">
                <Label>学期</Label>
                <Select
                  value={calendarId}
                  onValueChange={handleCalendarChange}
                >
                  <SelectTrigger aria-label="学期"><SelectValue placeholder="选择学期" /></SelectTrigger>
                  <SelectContent>
                    {(calendars.data ?? []).map((calendar) => calendar.id ? (
                      <SelectItem key={calendar.id} value={calendar.id}>{calendar.name}</SelectItem>
                    ) : null)}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>年级</Label>
                <Select
                  value={grade}
                  onValueChange={(value) => { setGrade(value); setMajorId(""); }}
                  disabled={!calendarId}
                >
                  <SelectTrigger aria-label="年级"><SelectValue placeholder="选择年级" /></SelectTrigger>
                  <SelectContent>
                    {(grades.data ?? []).map((item) => <SelectItem key={item} value={item}>{item}</SelectItem>)}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>专业</Label>
                <Select value={majorId} onValueChange={setMajorId} disabled={!grade}>
                  <SelectTrigger aria-label="专业"><SelectValue placeholder="选择专业" /></SelectTrigger>
                  <SelectContent>
                    {(majors.data ?? []).map((major) => major.id ? (
                      <SelectItem key={major.id} value={major.id}>{major.name}</SelectItem>
                    ) : null)}
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>时间筛选</CardTitle>
              <CardDescription>星期、起止节必须同时选择；未知时段仍会保留并明确标记。</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-2">
                <Label>星期</Label>
                <Select value={weekday} onValueChange={setWeekday}>
                  <SelectTrigger aria-label="星期"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="any">不限</SelectItem>
                    {weekdays.map((label, index) => <SelectItem key={label} value={String(index + 1)}>{label}</SelectItem>)}
                  </SelectContent>
                </Select>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div className="space-y-2">
                  <Label>起始节</Label>
                  <Select value={startSlot} onValueChange={setStartSlot}>
                    <SelectTrigger aria-label="起始节"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="any">不限</SelectItem>
                      {Array.from({ length: 20 }, (_, index) => index + 1).map((slot) => <SelectItem key={slot} value={String(slot)}>{slot}</SelectItem>)}
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label>结束节</Label>
                  <Select value={endSlot} onValueChange={setEndSlot}>
                    <SelectTrigger aria-label="结束节"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="any">不限</SelectItem>
                      {Array.from({ length: 20 }, (_, index) => index + 1).map((slot) => <SelectItem key={slot} value={String(slot)}>{slot}</SelectItem>)}
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="space-y-2">
                <Label>周次</Label>
                <Select value={week} onValueChange={setWeek}>
                  <SelectTrigger aria-label="周次"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="any">不限</SelectItem>
                    {Array.from({ length: 30 }, (_, index) => index + 1).map((value) => <SelectItem key={value} value={String(value)}>第 {value} 周</SelectItem>)}
                  </SelectContent>
                </Select>
              </div>
              <Button
                variant="ghost"
                className="w-full"
                onClick={() => { setWeekday("any"); setStartSlot("any"); setEndSlot("any"); setWeek("any"); }}
              >
                重置时间筛选
              </Button>
              {!timeComplete && timeFilterTouched ? (
                <p className="text-xs text-destructive" role="status">请完整选择星期、起始节和结束节。</p>
              ) : null}
              {timeComplete && !timeRangeValid ? (
                <p className="text-xs text-destructive" role="status">起始节不能晚于结束节。</p>
              ) : null}
            </CardContent>
          </Card>
        </aside>

        <main className="space-y-5">
          <section className={cn(mobileView !== "browse" && "hidden xl:block")} aria-labelledby="offering-results-heading">
            <h2 id="offering-results-heading" className="sr-only">教学班查询结果</h2>
            <Tabs value={activeTab} onValueChange={setActiveTab}>
              <TabsList className="mb-4 w-full justify-start overflow-x-auto">
                <TabsTrigger value="major">培养方案</TabsTrigger>
                <TabsTrigger value="nature">按性质</TabsTrigger>
                <TabsTrigger value="search">搜索</TabsTrigger>
              </TabsList>
              <TabsContent value="major">
                {resultsContent}
              </TabsContent>
              <TabsContent value="nature" className="space-y-3">
                <div className="max-w-sm space-y-2">
                  <Label>课程性质</Label>
                  <Select value={natureId} onValueChange={setNatureId}>
                    <SelectTrigger aria-label="课程性质"><SelectValue placeholder="选择课程性质" /></SelectTrigger>
                    <SelectContent>
                      {(natures.data ?? []).map((nature) => nature.id ? <SelectItem key={nature.id} value={nature.id}>{nature.name}</SelectItem> : null)}
                    </SelectContent>
                  </Select>
                </div>
                {resultsContent}
              </TabsContent>
              <TabsContent value="search" className="space-y-3">
                <div className="relative">
                  <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    value={searchText}
                    onChange={(event) => setSearchText(event.target.value)}
                    placeholder="课程名、课号、教师或拼音"
                    className="pl-9"
                    aria-label="搜索教学班"
                  />
                </div>
                {searchText.trim() !== debouncedSearch ? <p className="text-xs text-muted-foreground">正在等待输入完成…</p> : null}
                {resultsContent}
              </TabsContent>
            </Tabs>
          </section>

          <section className={cn(mobileView !== "schedule" && "hidden xl:block")} aria-labelledby="schedule-preview-heading">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <h2 id="schedule-preview-heading" className="font-semibold">课表预览</h2>
              <p className="text-sm text-muted-foreground">{staged.length} 个教学班 · {totalCredits.toFixed(1)} 学分</p>
            </div>
            {staged.length === 0 ? (
              <EmptyState title="待选课表为空" description="从教学班结果中加入课程后，会在这里按周次检查冲突。" />
            ) : (
              <>
                <div className="hidden md:block"><Timetable staged={staged} onSelect={setSelectedCourse} /></div>
                <MobileDaySchedule staged={staged} onSelect={setSelectedCourse} />
              </>
            )}
          </section>
        </main>

        <aside aria-label="待选教学班" className={cn("space-y-4", mobileView !== "selected" && "hidden xl:block")}>
          <Card>
            <CardHeader>
              <CardTitle>待选教学班</CardTitle>
              <CardDescription>按环境、账号和学期隔离保存在本机；不会写回教务系统。</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              {staged.length === 0 ? (
                <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">还没有加入教学班</p>
              ) : staged.map((item) => (
                <div key={offeringKey(item.course)} className="rounded-lg border p-3">
                  <div className="flex items-start justify-between gap-2">
                    <button type="button" className="min-w-0 text-left" onClick={() => setSelectedCourse(item)}>
                      <span className="font-medium">{item.course.name}</span>
                      <span className="mt-1 block text-xs text-muted-foreground">
                        {item.course.code} · {item.course.teachingClassCode ?? item.course.offeringId}
                      </span>
                      <span className="mt-1 block text-xs text-muted-foreground">{teachers(item.course)}</span>
                    </button>
                    <Button
                      size="icon"
                      variant="ghost"
                      onClick={() => removeCourse(offeringKey(item.course))}
                      aria-label={`移除${item.course.name}`}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                  <div className="mt-2 space-y-1 text-xs text-muted-foreground">
                    {item.course.scheduleUnknown ? (
                      <p className="rounded bg-destructive/10 px-2 py-1 text-destructive">时段未知，只能标为可能冲突</p>
                    ) : item.timeslots.map((slot, index) => (
                      <p key={`${slot.weekday}-${slot.startSlot}-${index}`} className="rounded bg-muted px-2 py-1">
                        {slotLabel(slot)}
                      </p>
                    ))}
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>
        </aside>
      </div>

      <Dialog open={Boolean(pendingAdd)} onOpenChange={(open) => !open && setPendingAdd(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2"><AlertTriangle className="h-5 w-5 text-destructive" />发现课表冲突</DialogTitle>
            <DialogDescription>
              {pendingHasConfirmedConflict
                ? "已确认冲突不能强制加入；请先调整待选教学班。"
                : "周次或时段信息不完整，只能在确认风险后加入并标记。"}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            {(pendingAdd?.conflicts ?? []).map((conflict) => (
              <div key={conflict.withOfferingId} className="flex items-center justify-between gap-3 rounded-lg border p-3">
                <div>
                  <p className="font-medium">{conflict.withName}</p>
                  <p className="text-xs text-muted-foreground">{conflict.withCode}</p>
                </div>
                <Badge variant={conflict.certainty === "confirmed" ? "destructive" : "outline"}>
                  {conflict.certainty === "confirmed" ? "已确认冲突" : "可能冲突"}
                </Badge>
              </div>
            ))}
          </div>
          <DialogFooter>
            <DialogClose asChild><Button variant="outline">取消</Button></DialogClose>
            {pendingHasConfirmedConflict ? null : (
              <Button variant="destructive" onClick={confirmConflictingAdd}>
                仍然加入并标记
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={clearOpen} onOpenChange={setClearOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>清空当前学期待选课表？</DialogTitle>
            <DialogDescription>只清除当前环境、账号和学期的本机数据，其他学期不会受影响。</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild><Button variant="outline">取消</Button></DialogClose>
            <Button
              variant="destructive"
              onClick={() => { clearSchedule(); setClearOpen(false); toast.success("当前学期待选课表已清空"); }}
            >
              确认清空
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(selectedCourse)} onOpenChange={(open) => !open && setSelectedCourse(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{selectedCourse?.course.name}</DialogTitle>
            <DialogDescription>
              {selectedCourse?.course.code} · 教学班 {selectedCourse?.course.teachingClassCode ?? selectedCourse?.course.offeringId}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2 text-sm">
            <p>{selectedCourse ? teachers(selectedCourse.course) : null}</p>
            {(selectedCourse?.timeslots ?? []).map((slot, index) => (
              <p key={`${slot.weekday}-${slot.startSlot}-${index}`} className="rounded-lg bg-muted p-3">{slotLabel(slot)}</p>
            ))}
            {selectedCourse?.course.scheduleUnknown ? (
              <p className="rounded-lg bg-destructive/10 p-3 text-destructive">上游没有可信时段，任何“无冲突”结论都不成立。</p>
            ) : null}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
