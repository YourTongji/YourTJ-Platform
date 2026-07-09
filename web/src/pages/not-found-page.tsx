import { Link } from "react-router";

import { EmptyState } from "@/components/common/states";
import { Button } from "@/components/ui/button";

export function NotFoundPage() {
  return (
    <EmptyState
      title="页面不存在"
      description="检查链接是否正确，或返回首页继续浏览。"
      action={<Button asChild><Link to="/">返回首页</Link></Button>}
    />
  );
}
