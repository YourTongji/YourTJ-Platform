import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  BookOpen,
  Check,
  Eye,
  FileWarning,
  Pencil,
  Plus,
  ShieldCheck,
  Tags,
  Trash2,
  X,
} from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import {
  AdminSectionHeader,
  AdminStatusBadge,
  PaginationControls,
  ReasonDialog,
} from "@/components/admin/admin-primitives";
import { ADMIN_CAPABILITIES, hasCapability } from "@/components/admin/capabilities";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { ApiError } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import type {
  AdminCourseCreateInput,
  AdminCourseUpdateInput,
  Board,
  Course,
  Tag,
  Upload,
  WatchedWord,
} from "@/lib/api/types";
import { formatNumber, formatRating, formatUnixTime } from "@/lib/format";

function MediaQueue() {
  const queryClient = useQueryClient();
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [status, setStatus] = React.useState<"pending" | "clean" | "quarantined" | "blocked">("pending");
  const [decision, setDecision] = React.useState<{ upload: Upload; action: "approve" | "block" } | null>(null);
  const [previewDecision, setPreviewDecision] = React.useState<Upload | null>(null);
  const [previewObject, setPreviewObject] = React.useState<{ uploadId: string; url: string } | null>(null);
  const cursor = cursorStack.at(-1);
  const uploads = useQuery({
    queryKey: ["admin", "media", status, cursor],
    queryFn: () => api.adminMediaUploads(cursor, status),
  });
  const moderate = useMutation({
    mutationFn: ({ id, action, reason }: { id: string; action: "approve" | "block"; reason: string }) =>
      api.moderateAdminMediaUpload(id, action, reason),
    onSuccess: async (_data, variables) => {
      toast.success(variables.action === "block" ? "媒体已隔离并进入删除队列" : "媒体已批准");
      setDecision(null);
      setPreviewObject(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "media"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "媒体审核失败"),
  });
  const preview = useMutation({
    mutationFn: async ({ uploadId, reason }: { uploadId: string; reason: string }) => {
      const grant = await api.createAdminMediaPreviewGrant(uploadId, reason);
      return { uploadId, blob: await api.adminMediaPreview(uploadId, grant.token) };
    },
    onSuccess: async ({ uploadId, blob }) => {
      setPreviewObject({ uploadId, url: URL.createObjectURL(blob) });
      setPreviewDecision(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "media"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "媒体预览失败"),
  });

  React.useEffect(() => {
    if (!previewObject) return;
    return () => URL.revokeObjectURL(previewObject.url);
  }, [previewObject]);

  return (
    <div className="space-y-3">
      <Card className="border-primary/30 bg-primary/5">
        <CardContent className="p-4 text-xs leading-5 text-muted-foreground">
          图片必须由当前审核员完成一次安全预览后才能批准；通用文件在恶意软件与沙箱扫描接入前不能批准。阻止会先隔离公开访问，再由持久任务删除 OSS 对象。界面不会暴露 object key、hash 或持久 URL。
        </CardContent>
      </Card>
      <div className="grid grid-cols-2 gap-1 rounded-lg bg-muted p-1 sm:grid-cols-4" role="group" aria-label="媒体状态筛选">
        {([
          ["pending", "待审核"],
          ["clean", "已发布"],
          ["quarantined", "删除中"],
          ["blocked", "已阻止"],
        ] as const).map(([value, label]) => (
          <Button
            key={value}
            type="button"
            size="sm"
            variant={status === value ? "secondary" : "ghost"}
            aria-pressed={status === value}
            onClick={() => {
              setStatus(value);
              setCursorStack([null]);
              setDecision(null);
              setPreviewObject(null);
            }}
          >
            {label}
          </Button>
        ))}
      </div>
      {uploads.isLoading ? <LoadingState label="加载媒体审核队列" /> : null}
      {uploads.isError ? <ErrorState error={uploads.error} onRetry={() => void uploads.refetch()} /> : null}
      {!uploads.isLoading && !uploads.isError && (uploads.data?.items ?? []).length === 0 ? (
        <EmptyState title="当前状态下没有可审核媒体" />
      ) : null}
      {uploads.data?.items?.map((upload) => (
        <Card key={upload.id}>
          <CardContent className="flex flex-col gap-3 p-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <FileWarning className="size-4 text-primary" aria-hidden="true" />
                <AdminStatusBadge value={upload.status} />
                <Badge variant="outline">{upload.kind ?? "file"}</Badge>
                {upload.deletionState ? <Badge variant="outline">删除 {upload.deletionState}</Badge> : null}
                <span className="text-xs text-muted-foreground">{upload.mime} · {formatNumber(upload.bytes)} B</span>
              </div>
              <p className="mt-2 truncate text-sm">上传 #{upload.id} · 账号 {upload.accountId}</p>
              <p className="mt-1 text-xs text-muted-foreground">
                {formatUnixTime(upload.createdAt)}
                {upload.usage ? ` · 用途 ${upload.usage}` : ""}
                {upload.imageWidth && upload.imageHeight ? ` · ${upload.imageWidth}×${upload.imageHeight}` : ""}
              </p>
              {previewObject?.uploadId === upload.id ? (
                <div className="mt-3 rounded-lg border bg-muted/30 p-2">
                  <img
                    src={previewObject.url}
                    alt={`待审上传 ${upload.id} 的一次性预览`}
                    className="max-h-80 max-w-full rounded object-contain"
                  />
                  <Button type="button" size="sm" variant="ghost" className="mt-2" onClick={() => setPreviewObject(null)}>
                    <X className="size-4" />关闭预览
                  </Button>
                </div>
              ) : null}
            </div>
            <div className="flex flex-wrap gap-2">
              {upload.status === "pending" && upload.kind === "image" ? (
                <Button type="button" size="sm" variant="outline" onClick={() => setPreviewDecision(upload)}>
                  <Eye className="size-4" />安全预览
                </Button>
              ) : null}
              {upload.status === "pending" && upload.kind !== "image" ? (
                <span className="self-center text-xs text-muted-foreground">等待扫描器，暂不可批准</span>
              ) : null}
              {upload.status === "pending" ? (
                <Button
                  type="button"
                  size="sm"
                  disabled={upload.approvalRequirement !== "satisfied"}
                  title={upload.approvalRequirement === "image_preview" ? "请先完成安全预览" : undefined}
                  onClick={() => setDecision({ upload, action: "approve" })}
                >
                  <Check className="size-4" />批准
                </Button>
              ) : null}
              {upload.status === "pending" || upload.status === "clean" ? (
                <Button type="button" variant="destructive" size="sm" onClick={() => setDecision({ upload, action: "block" })}><X className="size-4" />隔离并删除</Button>
              ) : null}
              {upload.status === "quarantined" && upload.deletionState === "dead_letter" ? (
                <Button type="button" variant="destructive" size="sm" onClick={() => setDecision({ upload, action: "block" })}><X className="size-4" />重试删除</Button>
              ) : null}
            </div>
          </CardContent>
        </Card>
      ))}
      <PaginationControls
        hasPrevious={cursorStack.length > 1}
        hasMore={Boolean(uploads.data?.hasMore && uploads.data.nextCursor)}
        onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
        onNext={() => uploads.data?.nextCursor && setCursorStack((items) => [...items, uploads.data?.nextCursor ?? null])}
      />
      <ReasonDialog
        open={Boolean(previewDecision)}
        onOpenChange={(open) => !open && setPreviewDecision(null)}
        title="读取待审媒体证据"
        description="系统会签发仅供当前管理员使用的 60 秒一次性授权，通过同源代理读取一张受 MIME 与字节上限保护的图片；读取原因会进入审计。"
        confirmLabel="生成并读取预览"
        isPending={preview.isPending}
        onConfirm={(reason) => previewDecision?.id && preview.mutate({ uploadId: previewDecision.id, reason })}
      />
      <ReasonDialog
        open={Boolean(decision)}
        onOpenChange={(open) => !open && setDecision(null)}
        title={decision?.action === "approve" ? "批准媒体对象" : "阻止媒体对象"}
        description={decision?.action === "approve"
          ? "只有完成当前审核员安全预览的图片才能批准；决定和原因会进入治理审计。"
          : "对象会立即进入不可公开访问的隔离状态，随后由持久任务删除 OSS 对象；失败会自动重试并可人工重新排队。"}
        confirmLabel={decision?.action === "approve" ? "确认批准" : "确认阻止"}
        destructive={decision?.action === "block"}
        isPending={moderate.isPending}
        onConfirm={(reason) => decision?.upload.id && moderate.mutate({ id: decision.upload.id, action: decision.action, reason })}
      />
    </div>
  );
}

interface CourseDraft {
  code: string;
  name: string;
  credit: string;
  department: string;
  teacherName: string;
}

interface CourseEditorState {
  course: Course | null;
  draft: CourseDraft;
}

const emptyCourseDraft: CourseDraft = {
  code: "",
  name: "",
  credit: "",
  department: "",
  teacherName: "",
};

function courseDraft(course?: Course): CourseDraft {
  return {
    code: course?.code ?? "",
    name: course?.name ?? "",
    credit: course?.credit === undefined ? "" : String(course.credit),
    department: course?.department ?? "",
    teacherName: course?.teacherName ?? "",
  };
}

function optionalText(value: string) {
  return value.trim() || undefined;
}

function parsedCredit(value: string) {
  if (!value.trim()) return undefined;
  return Number(value);
}

function isCreditValid(value: string) {
  const credit = parsedCredit(value);
  return credit === undefined || (Number.isFinite(credit) && credit >= 0 && credit <= 100);
}

function isCourseDraftValid(draft: CourseDraft) {
  return draft.code.trim().length >= 1
    && draft.code.trim().length <= 64
    && draft.name.trim().length >= 1
    && draft.name.trim().length <= 200
    && draft.department.trim().length <= 200
    && draft.teacherName.trim().length <= 200
    && isCreditValid(draft.credit);
}

function createCourseInput(draft: CourseDraft, reason: string): AdminCourseCreateInput {
  return {
    code: draft.code.trim(),
    name: draft.name.trim(),
    credit: parsedCredit(draft.credit),
    department: optionalText(draft.department),
    teacherName: optionalText(draft.teacherName),
    reason,
  };
}

function updateCourseInput(course: Course, draft: CourseDraft, reason: string) {
  const input: AdminCourseUpdateInput = { reason };
  const code = draft.code.trim();
  const name = draft.name.trim();
  const credit = parsedCredit(draft.credit);
  const department = optionalText(draft.department);
  const teacherName = optionalText(draft.teacherName);

  if (code !== (course.code ?? "").trim()) input.code = code;
  if (name !== (course.name ?? "").trim()) input.name = name;
  if (credit !== undefined && credit !== course.credit) input.credit = credit;
  if (department && department !== optionalText(course.department ?? "")) {
    input.department = department;
  }
  if (teacherName && teacherName !== optionalText(course.teacherName ?? "")) {
    input.teacherName = teacherName;
  }
  return input;
}

function hasCourseChanges(course: Course, draft: CourseDraft) {
  return Object.keys(updateCourseInput(course, draft, "reason")).length > 1;
}

function CoursesList() {
  const queryClient = useQueryClient();
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [editor, setEditor] = React.useState<CourseEditorState | null>(null);
  const [deleting, setDeleting] = React.useState<Course | null>(null);
  const [deleteConflict, setDeleteConflict] = React.useState<string | null>(null);
  const cursor = cursorStack.at(-1);
  const courses = useQuery({ queryKey: ["admin", "courses", cursor], queryFn: () => api.adminCourses(cursor) });
  const save = useMutation({
    mutationFn: ({ state, reason }: { state: CourseEditorState; reason: string }) => {
      if (state.course?.id) {
        return api.updateAdminCourse(
          state.course.id,
          updateCourseInput(state.course, state.draft, reason),
        );
      }
      return api.createAdminCourse(createCourseInput(state.draft, reason));
    },
    onSuccess: async (_, variables) => {
      toast.success(variables.state.course ? "课程资料已更新" : "课程已创建");
      if (!variables.state.course) setCursorStack([null]);
      setEditor(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "courses"] }),
        queryClient.invalidateQueries({ queryKey: ["courses"] }),
        variables.state.course?.id
          ? queryClient.invalidateQueries({ queryKey: ["course", variables.state.course.id] })
          : Promise.resolve(),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "课程保存失败"),
  });
  const remove = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => api.deleteAdminCourse(id, reason),
    onSuccess: async () => {
      toast.success("空课程已删除");
      setDeleting(null);
      setDeleteConflict(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "courses"] }),
        queryClient.invalidateQueries({ queryKey: ["courses"] }),
      ]);
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 409) {
        setDeleteConflict("该课程已有点评。为保留社区历史，课程不能删除；请改为修正课程资料。");
        return;
      }
      toast.error(error instanceof Error ? error.message : "课程删除失败");
    },
  });

  const editorIsValid = Boolean(editor && isCourseDraftValid(editor.draft));
  const editorCreditIsValid = !editor || isCreditValid(editor.draft.credit);
  const editorHasChanges = Boolean(
    editor && (!editor.course || hasCourseChanges(editor.course, editor.draft)),
  );

  function updateEditor(field: keyof CourseDraft, value: string) {
    setEditor((current) => current
      ? { ...current, draft: { ...current.draft, [field]: value } }
      : current);
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h3 className="font-semibold">课程目录管理</h3>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            创建和修改会记录原因；只有没有任何点评的空课程可以删除。
          </p>
        </div>
        <Button
          type="button"
          size="sm"
          onClick={() => setEditor({ course: null, draft: { ...emptyCourseDraft } })}
        >
          <Plus className="size-4" />新增课程
        </Button>
      </div>

      {courses.isLoading ? (
        <LoadingState label="加载课程目录" />
      ) : courses.isError ? (
        <ErrorState error={courses.error} onRetry={() => void courses.refetch()} />
      ) : (courses.data?.items ?? []).length === 0 ? (
        <EmptyState title="课程目录为空" description="可创建第一门课程，随后再关联教师和课程内容。" />
      ) : courses.data?.items?.map((course) => {
        const reviewCount = course.reviewCount ?? 0;
        return (
          <Card key={course.id ?? `${course.code}-${course.name}`}>
            <CardContent className="grid gap-3 p-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <BookOpen className="size-4 text-primary" aria-hidden="true" />
                  <span className="font-medium">{course.name}</span>
                  <Badge variant="outline">{course.code}</Badge>
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  {course.department ?? "院系未知"} · {course.teacherName ?? "教师未知"} · {course.credit ?? 0} 学分
                </p>
              </div>
              <div className="flex flex-col items-start gap-2 sm:items-end">
                <p className="text-xs text-muted-foreground">
                  {formatNumber(reviewCount)} 条点评 · {formatRating(course.reviewAvg)} 分
                </p>
                <div className="flex flex-wrap gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => setEditor({ course, draft: courseDraft(course) })}
                  >
                    <Pencil className="size-4" />编辑
                  </Button>
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    disabled={!course.id || reviewCount > 0}
                    title={reviewCount > 0 ? "已有点评的课程必须保留" : undefined}
                    onClick={() => {
                      setDeleteConflict(null);
                      setDeleting(course);
                    }}
                  >
                    <Trash2 className="size-4" />删除
                  </Button>
                </div>
                {reviewCount > 0 ? <p className="text-[10px] text-muted-foreground">已有点评，删除已锁定</p> : null}
              </div>
            </CardContent>
          </Card>
        );
      })}
      <PaginationControls
        hasPrevious={cursorStack.length > 1}
        hasMore={Boolean(courses.data?.hasMore && courses.data.nextCursor)}
        onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
        onNext={() => courses.data?.nextCursor && setCursorStack((items) => [...items, courses.data?.nextCursor ?? null])}
      />

      <ReasonDialog
        open={Boolean(editor)}
        onOpenChange={(open) => !open && setEditor(null)}
        title={editor?.course ? "编辑课程资料" : "新增课程"}
        description="课程目录变更会影响搜索、点评和客户端展示，并与操作原因一起写入治理审计。"
        confirmLabel={editor?.course ? "保存课程" : "创建课程"}
        isPending={save.isPending}
        confirmDisabled={!editorIsValid || !editorHasChanges}
        onConfirm={(reason) => editor && save.mutate({ state: editor, reason })}
      >
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="admin-course-code">课程代码</Label>
            <Input
              id="admin-course-code"
              value={editor?.draft.code ?? ""}
              onChange={(event) => updateEditor("code", event.target.value)}
              maxLength={64}
              required
              placeholder="如 100001"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="admin-course-name">课程名称</Label>
            <Input
              id="admin-course-name"
              value={editor?.draft.name ?? ""}
              onChange={(event) => updateEditor("name", event.target.value)}
              maxLength={200}
              required
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="admin-course-credit">学分</Label>
            <Input
              id="admin-course-credit"
              type="number"
              min={0}
              max={100}
              step="0.1"
              value={editor?.draft.credit ?? ""}
              onChange={(event) => updateEditor("credit", event.target.value)}
              aria-invalid={!editorCreditIsValid}
              aria-describedby={!editorCreditIsValid ? "admin-course-credit-error" : undefined}
              placeholder="可选，0–100"
            />
            {!editorCreditIsValid ? <p id="admin-course-credit-error" className="text-xs text-destructive">学分必须是 0–100 之间的数字。</p> : null}
          </div>
          <div className="space-y-2">
            <Label htmlFor="admin-course-department">开课院系</Label>
            <Input
              id="admin-course-department"
              value={editor?.draft.department ?? ""}
              onChange={(event) => updateEditor("department", event.target.value)}
              maxLength={200}
              placeholder="可选"
            />
          </div>
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="admin-course-teacher">教师</Label>
            <Input
              id="admin-course-teacher"
              value={editor?.draft.teacherName ?? ""}
              onChange={(event) => updateEditor("teacherName", event.target.value)}
              maxLength={200}
              placeholder="可选"
            />
          </div>
          {editor?.course ? (
            <p className="text-xs leading-5 text-muted-foreground sm:col-span-2">
              只会提交发生变化的字段；当前接口不支持清空已有的学分、院系或教师字段。
            </p>
          ) : null}
        </div>
      </ReasonDialog>

      <ReasonDialog
        open={Boolean(deleting)}
        onOpenChange={(open) => {
          if (!open) {
            setDeleting(null);
            setDeleteConflict(null);
          }
        }}
        title={`删除课程“${deleting?.name ?? ""}”`}
        description="删除只适用于没有点评的空课程；服务端会再次检查，避免破坏社区历史。"
        confirmLabel="确认删除"
        destructive
        isPending={remove.isPending}
        onConfirm={(reason) => deleting?.id && remove.mutate({ id: deleting.id, reason })}
      >
        {deleteConflict ? (
          <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive" role="alert">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
            <p>{deleteConflict}</p>
          </div>
        ) : null}
      </ReasonDialog>
    </div>
  );
}

type ResourceDecision<T> = { action: "save" | "delete"; item: T | null };
type WatchedWordAction = "block" | "censor" | "queue";

function CommunityManager() {
  const queryClient = useQueryClient();
  const emptyBoard = {
    slug: "",
    name: "",
    description: "",
    position: "0",
    isLocked: false,
    minTrustToPost: "0",
    isQa: false,
  };
  const [board, setBoard] = React.useState(emptyBoard);
  const [editingBoard, setEditingBoard] = React.useState<Board | null>(null);
  const [boardDecision, setBoardDecision] = React.useState<ResourceDecision<Board> | null>(null);
  const [boardConflict, setBoardConflict] = React.useState<string | null>(null);
  const [tag, setTag] = React.useState({ slug: "", name: "", description: "" });
  const [editingTag, setEditingTag] = React.useState<Tag | null>(null);
  const [tagDecision, setTagDecision] = React.useState<ResourceDecision<Tag> | null>(null);
  const [word, setWord] = React.useState("");
  const [wordAction, setWordAction] = React.useState<WatchedWordAction>("queue");
  const [wordDecision, setWordDecision] = React.useState<ResourceDecision<WatchedWord> | null>(null);
  const boards = useQuery({ queryKey: ["forum", "boards"], queryFn: api.boards });
  const tags = useQuery({ queryKey: ["admin", "tags"], queryFn: api.adminTags });
  const watchedWords = useQuery({ queryKey: ["admin", "watched-words"], queryFn: api.adminWatchedWords });

  function resetBoardForm() {
    setBoard(emptyBoard);
    setEditingBoard(null);
  }

  function resetTagForm() {
    setTag({ slug: "", name: "", description: "" });
    setEditingTag(null);
  }

  const boardAction = useMutation({
    mutationFn: async ({ decision, reason }: { decision: ResourceDecision<Board>; reason: string }) => {
      if (decision.action === "delete") {
        if (!decision.item?.id) throw new Error("缺少板块 ID");
        await api.deleteAdminBoard(decision.item.id, reason);
      } else if (decision.item?.id) {
        await api.updateAdminBoard(decision.item.id, {
          slug: board.slug.trim(),
          name: board.name.trim(),
          description: board.description.trim(),
          position: Number(board.position),
          isLocked: board.isLocked,
          minTrustToPost: Number(board.minTrustToPost),
          isQa: board.isQa,
          reason,
        });
      } else {
        await api.createAdminBoard({
          slug: board.slug.trim(),
          name: board.name.trim(),
          description: board.description.trim() || undefined,
          position: Number(board.position),
          isLocked: board.isLocked,
          minTrustToPost: Number(board.minTrustToPost),
          isQa: board.isQa,
          reason,
        });
      }
    },
    onSuccess: async (_, variables) => {
      toast.success(variables.decision.action === "delete"
        ? "板块已删除"
        : variables.decision.item ? "板块已更新" : "板块已创建");
      setBoardDecision(null);
      setBoardConflict(null);
      resetBoardForm();
      await queryClient.invalidateQueries({ queryKey: ["forum", "boards"] });
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 409) {
        setBoardConflict("该板块仍有帖子或子板块，不能删除。请先移动内容并处理子板块。");
        return;
      }
      toast.error(error instanceof Error ? error.message : "板块操作失败");
    },
  });
  const tagAction = useMutation({
    mutationFn: async ({ decision, reason }: { decision: ResourceDecision<Tag>; reason: string }) => {
      if (decision.action === "delete") {
        if (!decision.item?.id) throw new Error("缺少标签 ID");
        await api.deleteAdminTag(decision.item.id, reason);
      } else if (decision.item?.id) {
        await api.updateAdminTag(decision.item.id, {
          slug: tag.slug.trim(),
          name: tag.name.trim(),
          description: tag.description.trim(),
          reason,
        });
      } else {
        await api.createAdminTag({
          slug: tag.slug.trim(),
          name: tag.name.trim(),
          description: tag.description.trim() || undefined,
          reason,
        });
      }
    },
    onSuccess: async (_, variables) => {
      toast.success(variables.decision.action === "delete"
        ? "标签已删除"
        : variables.decision.item ? "标签已更新" : "标签已创建");
      setTagDecision(null);
      resetTagForm();
      await queryClient.invalidateQueries({ queryKey: ["admin", "tags"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "标签操作失败"),
  });
  const watchedWordAction = useMutation({
    mutationFn: async ({ decision, reason }: { decision: ResourceDecision<WatchedWord>; reason: string }) => {
      if (decision.action === "delete") {
        if (!decision.item?.id) throw new Error("缺少关注词 ID");
        await api.deleteAdminWatchedWord(decision.item.id, reason);
      } else {
        await api.createAdminWatchedWord({ word: word.trim(), action: wordAction, reason });
      }
    },
    onSuccess: async (_, variables) => {
      toast.success(variables.decision.action === "delete" ? "关注词已删除" : "关注词已添加");
      setWordDecision(null);
      setWord("");
      setWordAction("queue");
      await queryClient.invalidateQueries({ queryKey: ["admin", "watched-words"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "关注词操作失败"),
  });

  const boardIsValid = board.slug.trim().length >= 1
    && board.slug.trim().length <= 64
    && board.name.trim().length >= 1
    && board.name.trim().length <= 100
    && board.description.trim().length <= 500
    && Number.isInteger(Number(board.position))
    && Number(board.position) >= 0;
  const boardHasChanges = !editingBoard
    || board.slug.trim() !== editingBoard.slug
    || board.name.trim() !== editingBoard.name
    || board.description.trim() !== (editingBoard.description ?? "").trim()
    || Number(board.position) !== editingBoard.position
    || board.isLocked !== editingBoard.isLocked
    || Number(board.minTrustToPost) !== editingBoard.minTrustToPost
    || board.isQa !== editingBoard.isQa;
  const tagIsValid = tag.slug.trim().length >= 1
    && tag.slug.trim().length <= 64
    && tag.name.trim().length >= 1
    && tag.name.trim().length <= 100
    && tag.description.trim().length <= 500;
  const tagHasChanges = !editingTag
    || tag.slug.trim() !== editingTag.slug
    || tag.name.trim() !== editingTag.name
    || tag.description.trim() !== (editingTag.description ?? "").trim();

  return (
    <div className="space-y-4">
      <Card className="border-primary/30 bg-primary/5">
        <CardContent className="p-4 text-xs leading-5 text-muted-foreground">
          社区结构的创建、修改和删除均要求说明原因，并与数据变更写入同一治理事务。
        </CardContent>
      </Card>
      <div className="grid gap-4 xl:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>板块</CardTitle>
            <CardDescription>公共讨论区结构；有帖子或子板块时禁止删除。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-1">
              <div>
                <Label htmlFor="board-slug">Slug</Label>
                <Input id="board-slug" className="mt-1" maxLength={64} value={board.slug} onChange={(event) => setBoard((value) => ({ ...value, slug: event.target.value }))} />
              </div>
              <div>
                <Label htmlFor="board-name">名称</Label>
                <Input id="board-name" className="mt-1" maxLength={100} value={board.name} onChange={(event) => setBoard((value) => ({ ...value, name: event.target.value }))} />
              </div>
              <div>
                <Label htmlFor="board-position">排序</Label>
                <Input id="board-position" className="mt-1" type="number" min={0} value={board.position} onChange={(event) => setBoard((value) => ({ ...value, position: event.target.value }))} />
              </div>
              <div>
                <Label htmlFor="board-trust">最低发帖等级</Label>
                <Select value={board.minTrustToPost} onValueChange={(value) => setBoard((state) => ({ ...state, minTrustToPost: value }))}>
                  <SelectTrigger id="board-trust" className="mt-1"><SelectValue /></SelectTrigger>
                  <SelectContent>{[0, 1, 2, 3].map((level) => <SelectItem key={level} value={String(level)}>等级 {level}</SelectItem>)}</SelectContent>
                </Select>
              </div>
            </div>
            <div>
              <Label htmlFor="board-description">说明</Label>
              <Textarea id="board-description" className="mt-1" maxLength={500} value={board.description} onChange={(event) => setBoard((value) => ({ ...value, description: event.target.value }))} />
            </div>
            <div className="flex flex-wrap gap-5 text-sm">
              <Label className="flex items-center gap-2"><Switch checked={board.isLocked} onCheckedChange={(checked) => setBoard((value) => ({ ...value, isLocked: checked }))} />锁定发帖</Label>
              <Label className="flex items-center gap-2"><Switch checked={board.isQa} onCheckedChange={(checked) => setBoard((value) => ({ ...value, isQa: checked }))} />问答板块</Label>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                size="sm"
                onClick={() => setBoardDecision({ action: "save", item: editingBoard })}
                disabled={!boardIsValid || !boardHasChanges || boardAction.isPending}
              >
                {editingBoard ? <Pencil className="size-4" /> : <Plus className="size-4" />}
                {editingBoard ? "保存板块" : "创建板块"}
              </Button>
              {editingBoard ? <Button type="button" variant="ghost" size="sm" onClick={resetBoardForm}>取消编辑</Button> : null}
            </div>
            {boards.isLoading ? <LoadingState /> : boards.isError ? <ErrorState error={boards.error} onRetry={() => void boards.refetch()} /> : (boards.data ?? []).length === 0 ? <EmptyState title="暂无板块" /> : (
              <div className="max-h-72 space-y-2 overflow-y-auto">
                {boards.data?.map((item) => (
                  <div key={item.id} className="rounded-lg border p-2 text-sm">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <p className="truncate font-medium">{item.name}</p>
                        <p className="truncate text-xs text-muted-foreground">/{item.slug} · {formatNumber(item.threadCount)} 个主题</p>
                        <div className="mt-1 flex flex-wrap gap-1">
                          {item.isLocked ? <Badge variant="outline">已锁定</Badge> : null}
                          {(item.minTrustToPost ?? 0) > 0 ? <Badge variant="outline">等级 {item.minTrustToPost}+</Badge> : null}
                          {item.isQa ? <Badge variant="outline">问答</Badge> : null}
                        </div>
                      </div>
                      <div className="flex shrink-0 gap-1">
                        <Button type="button" variant="ghost" size="sm" onClick={() => { setEditingBoard(item); setBoard({ slug: item.slug ?? "", name: item.name ?? "", description: item.description ?? "", position: String(item.position ?? 0), isLocked: item.isLocked ?? false, minTrustToPost: String(item.minTrustToPost ?? 0), isQa: item.isQa ?? false }); }}><Pencil className="size-3.5" />编辑</Button>
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="text-destructive hover:text-destructive"
                          disabled={!item.id || (item.threadCount ?? 0) > 0}
                          title={(item.threadCount ?? 0) > 0 ? "有主题的板块不能删除" : undefined}
                          onClick={() => { setBoardConflict(null); setBoardDecision({ action: "delete", item }); }}
                        >
                          <Trash2 className="size-3.5" />删除
                        </Button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>标签</CardTitle>
            <CardDescription>论坛内容分类和检索入口。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="grid gap-2">
              <div><Label htmlFor="tag-slug">Slug</Label><Input id="tag-slug" className="mt-1" maxLength={64} value={tag.slug} onChange={(event) => setTag((value) => ({ ...value, slug: event.target.value }))} /></div>
              <div><Label htmlFor="tag-name">名称</Label><Input id="tag-name" className="mt-1" maxLength={100} value={tag.name} onChange={(event) => setTag((value) => ({ ...value, name: event.target.value }))} /></div>
              <div><Label htmlFor="tag-description">说明</Label><Textarea id="tag-description" className="mt-1" maxLength={500} value={tag.description} onChange={(event) => setTag((value) => ({ ...value, description: event.target.value }))} /></div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                size="sm"
                onClick={() => setTagDecision({ action: "save", item: editingTag })}
                disabled={!tagIsValid || !tagHasChanges || tagAction.isPending}
              >
                {editingTag ? <Pencil className="size-4" /> : <Plus className="size-4" />}
                {editingTag ? "保存标签" : "创建标签"}
              </Button>
              {editingTag ? <Button type="button" variant="ghost" size="sm" onClick={resetTagForm}>取消编辑</Button> : null}
            </div>
            {tags.isLoading ? <LoadingState /> : tags.isError ? <ErrorState error={tags.error} onRetry={() => void tags.refetch()} /> : (tags.data ?? []).length === 0 ? <EmptyState title="暂无标签" /> : (
              <div className="max-h-72 space-y-2 overflow-y-auto">
                {tags.data?.map((item) => (
                  <div key={item.id} className="flex items-start justify-between gap-2 rounded-lg border p-2 text-sm">
                    <div className="min-w-0">
                      <p className="truncate font-medium">#{item.name}</p>
                      <p className="truncate text-xs text-muted-foreground">/{item.slug} · {formatNumber(item.threadCount)} 个主题</p>
                    </div>
                    <div className="flex shrink-0 gap-1">
                      <Button type="button" variant="ghost" size="sm" onClick={() => { setEditingTag(item); setTag({ slug: item.slug ?? "", name: item.name ?? "", description: item.description ?? "" }); }}><Pencil className="size-3.5" />编辑</Button>
                      <Button type="button" variant="ghost" size="sm" className="text-destructive hover:text-destructive" disabled={!item.id} onClick={() => setTagDecision({ action: "delete", item })}><Trash2 className="size-3.5" />删除</Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>关注词</CardTitle>
            <CardDescription>阻止、替换或送审敏感内容；修改词条需删除后重新添加。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div><Label htmlFor="watched-word">词语</Label><Input id="watched-word" className="mt-1" maxLength={200} value={word} onChange={(event) => setWord(event.target.value)} /></div>
            <div>
              <Label htmlFor="watched-word-action">动作</Label>
              <Select value={wordAction} onValueChange={(value) => setWordAction(value as WatchedWordAction)}>
                <SelectTrigger id="watched-word-action" className="mt-1"><SelectValue /></SelectTrigger>
                <SelectContent><SelectItem value="queue">送审</SelectItem><SelectItem value="censor">替换</SelectItem><SelectItem value="block">阻止</SelectItem></SelectContent>
              </Select>
            </div>
            <Button
              type="button"
              size="sm"
              onClick={() => setWordDecision({ action: "save", item: null })}
              disabled={!word.trim() || word.trim().length > 200 || watchedWordAction.isPending}
            >
              <Plus className="size-4" />添加关注词
            </Button>
            {watchedWords.isLoading ? <LoadingState /> : watchedWords.isError ? <ErrorState error={watchedWords.error} onRetry={() => void watchedWords.refetch()} /> : (watchedWords.data ?? []).length === 0 ? <EmptyState title="暂无关注词" /> : (
              <div className="max-h-72 space-y-2 overflow-y-auto">
                {watchedWords.data?.map((item) => (
                  <div key={item.id} className="flex items-center justify-between gap-2 rounded-lg border p-2 text-sm">
                    <span className="min-w-0 truncate">{item.word}</span>
                    <div className="flex shrink-0 items-center gap-1">
                      <AdminStatusBadge value={item.action} />
                      <Button type="button" variant="ghost" size="sm" className="text-destructive hover:text-destructive" disabled={!item.id} onClick={() => setWordDecision({ action: "delete", item })}><Trash2 className="size-3.5" />删除</Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <ReasonDialog
        open={Boolean(boardDecision)}
        onOpenChange={(open) => { if (!open) { setBoardDecision(null); setBoardConflict(null); } }}
        title={boardDecision?.action === "delete" ? `删除板块“${boardDecision.item?.name ?? ""}”` : boardDecision?.item ? "保存板块修改" : "创建板块"}
        description={boardDecision?.action === "delete" ? "只有没有帖子和子板块的空板块可以删除。" : "板块结构会影响发帖入口和内容归属。"}
        confirmLabel={boardDecision?.action === "delete" ? "确认删除" : boardDecision?.item ? "确认保存" : "确认创建"}
        destructive={boardDecision?.action === "delete"}
        isPending={boardAction.isPending}
        onConfirm={(reason) => boardDecision && boardAction.mutate({ decision: boardDecision, reason })}
      >
        {boardConflict ? (
          <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive" role="alert">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
            <p>{boardConflict}</p>
          </div>
        ) : null}
      </ReasonDialog>
      <ReasonDialog
        open={Boolean(tagDecision)}
        onOpenChange={(open) => !open && setTagDecision(null)}
        title={tagDecision?.action === "delete" ? `删除标签“${tagDecision.item?.name ?? ""}”` : tagDecision?.item ? "保存标签修改" : "创建标签"}
        description={tagDecision?.action === "delete" ? "删除会解除该标签与现有主题的关联。" : "标签变更会影响论坛分类和检索。"}
        confirmLabel={tagDecision?.action === "delete" ? "确认删除" : tagDecision?.item ? "确认保存" : "确认创建"}
        destructive={tagDecision?.action === "delete"}
        isPending={tagAction.isPending}
        onConfirm={(reason) => tagDecision && tagAction.mutate({ decision: tagDecision, reason })}
      />
      <ReasonDialog
        open={Boolean(wordDecision)}
        onOpenChange={(open) => !open && setWordDecision(null)}
        title={wordDecision?.action === "delete" ? `删除关注词“${wordDecision.item?.word ?? ""}”` : "添加关注词"}
        description="词表变更会立即影响内容发布和审核队列，请避免录入口令或无关个人信息。"
        confirmLabel={wordDecision?.action === "delete" ? "确认删除" : "确认添加"}
        destructive={wordDecision?.action === "delete"}
        isPending={watchedWordAction.isPending}
        onConfirm={(reason) => wordDecision && watchedWordAction.mutate({ decision: wordDecision, reason })}
      />
    </div>
  );
}

export function ResourcesPanel({ capabilities }: { capabilities: Set<string> }) {
  const canModerateMedia = hasCapability(capabilities, ADMIN_CAPABILITIES.moderateContent);
  const canManageCourses = hasCapability(capabilities, ADMIN_CAPABILITIES.manageCourses);
  const canManageCommunity = hasCapability(capabilities, ADMIN_CAPABILITIES.manageCommunity);
  const defaultTab = canModerateMedia ? "media" : canManageCourses ? "courses" : "community";

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="内容与资源"
        description="集中处理待审媒体、课程目录和社区结构。每个标签页只在服务端签发对应能力时出现。"
      />
      <Tabs defaultValue={defaultTab}>
        <TabsList className="scrollbar-none h-auto max-w-full justify-start overflow-x-auto">
          {canModerateMedia ? <TabsTrigger value="media"><ShieldCheck className="mr-1 size-4" />待审媒体</TabsTrigger> : null}
          {canManageCourses ? <TabsTrigger value="courses"><BookOpen className="mr-1 size-4" />课程目录</TabsTrigger> : null}
          {canManageCommunity ? <TabsTrigger value="community"><Tags className="mr-1 size-4" />社区结构</TabsTrigger> : null}
        </TabsList>
        {canModerateMedia ? <TabsContent value="media" className="pt-2"><MediaQueue /></TabsContent> : null}
        {canManageCourses ? <TabsContent value="courses" className="pt-2"><CoursesList /></TabsContent> : null}
        {canManageCommunity ? <TabsContent value="community" className="pt-2"><CommunityManager /></TabsContent> : null}
      </Tabs>
    </div>
  );
}
