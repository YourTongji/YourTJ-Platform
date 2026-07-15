import { useQuery } from "@tanstack/react-query";
import { CalendarDays, Download, Plus, Search, Trash2 } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { PageHeader } from "@/components/common/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
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
  type ScheduleConflict,
  type ScheduledCourse,
  type ScheduleScope,
  timetableCells,
  useScheduleStore,
} from "@/lib/schedule-store";
import { cn } from "@/lib/utils";

const weekdays = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
const maxSlot = 13;

interface PendingPossibleConflict {
  calendarId: string;
  course: SelectionCourse;
  timeslots: TimeSlot[];
  conflict: ScheduleConflict;
}

function teachingClassId(course: SelectionCourse) {
  return course.id;
}

function teachers(course: SelectionCourse) {
  const names = course.teacherNames?.length ? course.teacherNames : course.teacherName ? [course.teacherName] : [];
  return names.length ? names.join(" / ") : "教师待同步";
}

function CourseRow({
  course,
  onAdd,
}: {
  course: SelectionCourse;
  onAdd: (course: SelectionCourse) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border p-3">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <p className="font-medium">{course.name}</p>
          <Badge variant="secondary">{course.credit ?? 0} 学分</Badge>
        </div>
        <p className="mt-1 text-sm text-muted-foreground">
          {course.code} · {teachers(course)}
        </p>
      </div>
      <Button size="sm" onClick={() => onAdd(course)}>
        <Plus className="h-4 w-4" />
        加入
      </Button>
    </div>
  );
}

function Timetable({ staged }: { staged: ScheduledCourse[] }) {
  const cells = timetableCells(staged, maxSlot);
  return (
    <div className="overflow-x-auto rounded-lg border bg-card">
      <div className="grid min-w-[760px] grid-cols-[4rem_repeat(7,minmax(6rem,1fr))]">
        <div className="border-b border-r bg-muted p-2 text-center text-xs font-medium text-muted-foreground">
          节次
        </div>
        {weekdays.map((weekday) => (
          <div key={weekday} className="border-b border-r bg-muted p-2 text-center text-xs font-medium">
            {weekday}
          </div>
        ))}
        {Array.from({ length: maxSlot }, (_, index) => index + 1).map((section) => (
          <React.Fragment key={section}>
            <div className="border-b border-r p-2 text-center text-xs text-muted-foreground">
              {section}
            </div>
            {weekdays.map((_, weekdayIndex) => {
              const key = `${weekdayIndex + 1}-${section}`;
              return (
                <div key={key} className="min-h-16 border-b border-r p-1.5">
                  <div className="space-y-1">
                    {(cells[key] ?? []).map((item) => (
                      <div
                        key={`${teachingClassId(item.course)}-${section}`}
                        className="rounded px-2 py-1 text-xs leading-snug text-white"
                        style={{ backgroundColor: item.color }}
                      >
                        <p className="line-clamp-2 font-medium">{item.course.name}</p>
                        <p className="opacity-90">{teachers(item.course)}</p>
                      </div>
                    ))}
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

function exportCsv(staged: ScheduledCourse[]) {
  const rows = [
    ["code", "name", "credit", "teacher", "weekday", "startSlot", "endSlot", "weeks", "location"],
    ...staged.flatMap((item) =>
      item.timeslots.map((slot) => [
        item.course.code ?? "",
        item.course.name ?? "",
        String(item.course.credit ?? ""),
        slot.teacherName ?? item.course.teacherName ?? "",
        String(slot.weekday ?? ""),
        String(slot.startSlot ?? ""),
        String(slot.endSlot ?? ""),
        slot.weeks ?? "",
        slot.location ?? "",
      ]),
    ),
  ];
  const csv = rows.map((row) => row.map((cell) => `"${cell.replaceAll('"', '""')}"`).join(",")).join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "yourtj-schedule.csv";
  link.click();
  URL.revokeObjectURL(url);
}

export function SchedulePage() {
  const { account } = useAuth();
  const [calendarId, setCalendarId] = React.useState("");
  const [grade, setGrade] = React.useState("");
  const [majorId, setMajorId] = React.useState("");
  const [natureId, setNatureId] = React.useState("");
  const [searchText, setSearchText] = React.useState("");
  const [activeTab, setActiveTab] = React.useState("major");
  const [pendingPossibleConflict, setPendingPossibleConflict] =
    React.useState<PendingPossibleConflict | null>(null);
  const calendarIdRef = React.useRef(calendarId);
  calendarIdRef.current = calendarId;
  const scheduleScope = React.useMemo<ScheduleScope>(
    () => ({
      environment: API_BASE_URL,
      principal: account?.id ?? "anonymous",
      calendarId,
    }),
    [account?.id, calendarId],
  );
  const staged = useScheduleStore(scheduleScope, (state) => state.staged);
  const addCourse = useScheduleStore(scheduleScope, (state) => state.addCourse);
  const removeCourse = useScheduleStore(scheduleScope, (state) => state.removeCourse);
  const clearSchedule = useScheduleStore(scheduleScope, (state) => state.clear);

  const calendars = useQuery({ queryKey: ["selection", "calendars"], queryFn: api.calendars });
  const latest = useQuery({ queryKey: ["selection", "latest"], queryFn: api.selectionLatestUpdate });
  const grades = useQuery({
    queryKey: ["selection", "grades", calendarId],
    queryFn: () => api.grades(calendarId),
    enabled: Boolean(calendarId),
  });
  const majors = useQuery({
    queryKey: ["selection", "majors", calendarId, grade],
    queryFn: () => api.majors(calendarId, grade),
    enabled: Boolean(calendarId && grade),
  });
  const natures = useQuery({ queryKey: ["selection", "natures"], queryFn: api.courseNatures });
  const majorCourses = useQuery({
    queryKey: ["selection", "major-courses", calendarId, majorId, grade],
    queryFn: () => api.selectionByMajor(calendarId, majorId, grade),
    enabled: Boolean(calendarId && majorId && grade),
  });
  const natureCourses = useQuery({
    queryKey: ["selection", "nature-courses", calendarId, natureId],
    queryFn: () => api.selectionByNature(calendarId, natureId),
    enabled: Boolean(calendarId && natureId),
  });
  const searchCourses = useQuery({
    queryKey: ["selection", "search", calendarId, searchText.trim()],
    queryFn: () => api.selectionSearch(calendarId, searchText.trim()),
    enabled: Boolean(calendarId) && searchText.trim().length >= 2,
  });

  React.useEffect(() => {
    const current = calendars.data?.find((calendar) => calendar.isCurrent);
    if (!calendarId && current?.id) {
      calendarIdRef.current = current.id;
      setCalendarId(current.id);
    }
  }, [calendarId, calendars.data]);

  async function handleAddCourse(course: SelectionCourse) {
    const selectedCalendarId = calendarId;
    if (course.calendarId !== selectedCalendarId) {
      toast.error("教学班不属于当前学期，请刷新后重试");
      return;
    }
    try {
      const slots = await api.selectionTimeslots(teachingClassId(course));
      if (calendarIdRef.current !== selectedCalendarId) {
        return;
      }
      const result = addCourse(course, slots);
      if (result.status === "added") {
        toast.success(`${course.name} 已加入课表`);
      } else if (result.status === "duplicate") {
        toast.info(`${course.name} 已在当前课表中`);
      } else if (result.status === "scopeMismatch") {
        toast.error("教学班不属于当前学期，请刷新后重试");
      } else if (result.conflict.kind === "confirmed") {
        toast.error(`与 ${result.conflict.withName} 在相同周次确定冲突`);
      } else {
        setPendingPossibleConflict({
          calendarId: selectedCalendarId,
          course,
          timeslots: slots,
          conflict: result.conflict,
        });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "拉取课程时段失败");
    }
  }

  function confirmPossibleConflict() {
    const pending = pendingPossibleConflict;
    if (!pending) {
      return;
    }
    if (pending.calendarId !== calendarId) {
      setPendingPossibleConflict(null);
      toast.error("学期已切换，请重新选择教学班");
      return;
    }
    const result = addCourse(pending.course, pending.timeslots, {
      allowPossibleConflict: true,
    });
    setPendingPossibleConflict(null);
    if (result.status === "added") {
      toast.success(`${pending.course.name} 已加入课表`);
    } else if (result.status === "duplicate") {
      toast.info(`${pending.course.name} 已在当前课表中`);
    } else if (result.status === "scopeMismatch") {
      toast.error("教学班不属于当前学期，请刷新后重试");
    } else {
      toast.error(`与 ${result.conflict.withName} 在相同周次确定冲突`);
    }
  }

  function handleCalendarChange(nextCalendarId: string) {
    calendarIdRef.current = nextCalendarId;
    setCalendarId(nextCalendarId);
    setGrade("");
    setMajorId("");
    setNatureId("");
    setSearchText("");
    setPendingPossibleConflict(null);
  }

  const totalCredits = staged.reduce((sum, item) => sum + Number(item.course.credit ?? 0), 0);

  if (calendars.isLoading) {
    return <LoadingState label="加载选课基础数据" />;
  }
  if (calendars.isError) {
    return <ErrorState error={calendars.error} onRetry={() => void calendars.refetch()} />;
  }

  return (
    <div>
      <Dialog
        open={pendingPossibleConflict !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingPossibleConflict(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>可能存在周次冲突</DialogTitle>
            <DialogDescription>
              {pendingPossibleConflict
                ? `与“${pendingPossibleConflict.conflict.withName}”在${weekdays[(pendingPossibleConflict.conflict.candidateSlot.weekday ?? 1) - 1]} ${pendingPossibleConflict.conflict.candidateSlot.startSlot}-${pendingPossibleConflict.conflict.candidateSlot.endSlot} 节重叠，但至少一方周次缺失或无法解析，系统不能确认是否同周上课。`
                : null}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPendingPossibleConflict(null)}>
              取消
            </Button>
            <Button onClick={confirmPossibleConflict}>仍然加入</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <PageHeader
        title="选课排课"
        description="按学期浏览教学班，完成本地待选、冲突检查和课表模拟。"
        actions={
          <>
            <Button variant="outline" onClick={() => exportCsv(staged)} disabled={staged.length === 0}>
              <Download className="h-4 w-4" />
              导出 CSV
            </Button>
            <Button variant="outline" onClick={clearSchedule} disabled={staged.length === 0}>
              <Trash2 className="h-4 w-4" />
              清空
            </Button>
          </>
        }
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_21rem]">
        <div className="space-y-5">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <CalendarDays className="h-5 w-5 text-primary" />
                选课上下文
              </CardTitle>
              <CardDescription>
                最近同步：{formatDate(latest.data?.updatedAt ?? null)}
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-3 md:grid-cols-3">
              <div className="space-y-2">
                <Label>学期</Label>
                <Select value={calendarId} onValueChange={handleCalendarChange}>
                  <SelectTrigger>
                    <SelectValue placeholder="选择学期" />
                  </SelectTrigger>
                  <SelectContent>
                    {(calendars.data ?? []).map((calendar) => (
                      <SelectItem key={calendar.id} value={calendar.id ?? ""}>
                        {calendar.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>年级</Label>
                <Select value={grade} onValueChange={(value) => { setGrade(value); setMajorId(""); }} disabled={!calendarId}>
                  <SelectTrigger>
                    <SelectValue placeholder="选择年级" />
                  </SelectTrigger>
                  <SelectContent>
                    {(grades.data ?? []).map((item) => (
                      <SelectItem key={item} value={item}>
                        {item}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>专业</Label>
                <Select value={majorId} onValueChange={setMajorId} disabled={!grade}>
                  <SelectTrigger>
                    <SelectValue placeholder="选择专业" />
                  </SelectTrigger>
                  <SelectContent>
                    {(majors.data ?? []).map((major) => (
                      <SelectItem key={major.id} value={major.id ?? ""}>
                        {major.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Tabs value={activeTab} onValueChange={setActiveTab}>
            <TabsList className="w-full justify-start overflow-x-auto">
              <TabsTrigger value="major">培养方案</TabsTrigger>
              <TabsTrigger value="nature">按性质</TabsTrigger>
              <TabsTrigger value="search">搜索</TabsTrigger>
            </TabsList>

            <TabsContent value="major" className="space-y-3">
              {!majorId ? (
                <EmptyState title="先选择年级和专业" description="选择后会读取该专业培养方案课程。" />
              ) : majorCourses.isLoading ? (
                <LoadingState />
              ) : majorCourses.isError ? (
                <ErrorState error={majorCourses.error} onRetry={() => void majorCourses.refetch()} />
              ) : (
                <div className="space-y-2">
                  {(majorCourses.data ?? []).map((course) => (
                    <CourseRow key={course.id} course={course} onAdd={handleAddCourse} />
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="nature" className="space-y-3">
              <div className="max-w-sm space-y-2">
                <Label>课程性质</Label>
                <Select value={natureId} onValueChange={setNatureId}>
                  <SelectTrigger>
                    <SelectValue placeholder="选择课程性质" />
                  </SelectTrigger>
                  <SelectContent>
                    {(natures.data ?? []).map((nature) => (
                      <SelectItem key={nature.id} value={nature.id ?? ""}>
                        {nature.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              {!natureId ? (
                <EmptyState title="选择一个课程性质" description="例如通识选修、专业选修等。" />
              ) : natureCourses.isLoading ? (
                <LoadingState />
              ) : natureCourses.isError ? (
                <ErrorState error={natureCourses.error} onRetry={() => void natureCourses.refetch()} />
              ) : (
                <div className="space-y-2">
                  {(natureCourses.data ?? []).map((course) => (
                    <CourseRow key={course.id} course={course} onAdd={handleAddCourse} />
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="search" className="space-y-3">
              <div className="relative">
                <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={searchText}
                  onChange={(event) => setSearchText(event.target.value)}
                  placeholder="输入课程名、课号或教师"
                  className="pl-9"
                />
              </div>
              {searchText.trim().length < 2 ? (
                <EmptyState title="输入关键词搜索" description="支持课程名、课号、教师名、拼音等关键词。" />
              ) : searchCourses.isLoading ? (
                <LoadingState />
              ) : searchCourses.isError ? (
                <ErrorState error={searchCourses.error} onRetry={() => void searchCourses.refetch()} />
              ) : (
                <div className="space-y-2">
                  {(searchCourses.data ?? []).map((course) => (
                    <CourseRow key={course.id} course={course} onAdd={handleAddCourse} />
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>

          <section>
            <div className="mb-3 flex items-center justify-between">
              <h2 className="font-semibold">课表预览</h2>
              <p className="text-sm text-muted-foreground">{staged.length} 门课程 · {totalCredits.toFixed(1)} 学分</p>
            </div>
            <Timetable staged={staged} />
          </section>
        </div>

        <aside className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>待选课程</CardTitle>
              <CardDescription>按当前环境、账号和学期保存在本机浏览器，不会写回一系统。</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              {staged.length === 0 ? (
                <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">还没有加入课程</p>
              ) : (
                staged.map((item) => (
                  <div key={teachingClassId(item.course)} className="rounded-md border p-3">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <p className="font-medium">{item.course.name}</p>
                        <p className="text-xs text-muted-foreground">{item.course.code} · {teachers(item.course)}</p>
                      </div>
                      <Button
                        size="icon"
                        variant="ghost"
                        onClick={() => removeCourse(teachingClassId(item.course))}
                        aria-label="移除课程"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                    <div className="mt-2 space-y-1 text-xs text-muted-foreground">
                      {item.timeslots.length === 0 ? (
                        <p>暂无时段</p>
                      ) : (
                        item.timeslots.map((slot, index) => (
                          <p key={index} className={cn("rounded px-2 py-1", "bg-muted")}>
                            {weekdays[(slot.weekday ?? 1) - 1]} {slot.startSlot}-{slot.endSlot} 节 · {slot.location ?? "地点待定"} · {slot.weeks ?? "周次待定"}
                          </p>
                        ))
                      )}
                    </div>
                  </div>
                ))
              )}
            </CardContent>
          </Card>
        </aside>
      </div>
    </div>
  );
}
