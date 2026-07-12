import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function SettingsSecurityPage() {
  return (
    <div className="mx-auto max-w-lg space-y-6">
      <Card>
        <CardHeader><CardTitle>安全设置</CardTitle></CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">密码管理、最近认证和账号安全选项。</p>
        </CardContent>
      </Card>
    </div>
  );
}
