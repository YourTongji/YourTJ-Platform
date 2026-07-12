import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function SettingsSessionsPage() {
  return (
    <div className="mx-auto max-w-lg space-y-6">
      <Card>
        <CardHeader><CardTitle>设备与会话管理</CardTitle></CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">查看和管理登录设备与活跃会话。</p>
        </CardContent>
      </Card>
    </div>
  );
}
