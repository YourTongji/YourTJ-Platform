import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Gift, KeyRound, Plus, ShieldCheck, ShoppingBag, Trash2, Wallet } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Product, Purchase, Task } from "@/lib/api/types";
import { formatDate, formatNumber, shortHash } from "@/lib/format";
import { randomUuid } from "@/lib/random";
import {
  buildClientSignedPayload,
  clearLocalWallet,
  createLocalWallet,
  getLocalWallet,
  signPayload,
} from "@/lib/wallet";

function signIntent(action: string, payload: unknown) {
  return signPayload(buildClientSignedPayload({ action, payload }));
}

function WalletSetup() {
  const queryClient = useQueryClient();
  const [wallet, setWallet] = React.useState(() => getLocalWallet());
  const bind = useMutation({
    mutationFn: (publicKey: string) => api.bindWallet(publicKey),
    onSuccess: async () => {
      toast.success("钱包公钥已绑定");
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "绑定失败"),
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="h-4 w-4 text-primary" />
          本地签名钱包
        </CardTitle>
        <CardDescription>
          私钥只保存在当前浏览器 localStorage；服务端只保存 Ed25519 公钥。
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {wallet ? (
          <div className="rounded-md border bg-muted/60 p-3 text-xs">
            <p className="font-medium">公钥</p>
            <p className="mt-1 break-all text-muted-foreground">{wallet.publicKey}</p>
          </div>
        ) : (
          <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">本机还没有钱包密钥。</p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            onClick={() => {
              const next = createLocalWallet();
              setWallet(next);
              toast.success("已生成本地钱包");
            }}
          >
            生成/重置本地钱包
          </Button>
          <Button
            onClick={() => wallet && bind.mutate(wallet.publicKey)}
            disabled={!wallet || bind.isPending}
          >
            绑定公钥
          </Button>
          <Button
            variant="outline"
            onClick={() => {
              clearLocalWallet();
              setWallet(null);
            }}
            disabled={!wallet}
          >
            <Trash2 className="h-4 w-4" />
            清除本机私钥
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function LegacyClaimPanel() {
  const queryClient = useQueryClient();
  const [challenge, setChallenge] = React.useState<{ challengeId?: string; nonce?: string } | null>(null);
  const [legacyUserHash, setLegacyUserHash] = React.useState("");
  const [signature, setSignature] = React.useState("");
  const getChallenge = useMutation({
    mutationFn: api.claimChallenge,
    onSuccess: (data) => {
      setChallenge(data);
      toast.success("认领挑战已生成，10 分钟内有效");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "获取挑战失败"),
  });
  const claim = useMutation({
    mutationFn: () => {
      if (!challenge?.challengeId) {
        throw new Error("请先获取挑战");
      }
      return api.claimWallet({ legacyUserHash, challengeId: challenge.challengeId, signature });
    },
    onSuccess: async () => {
      toast.success("旧钱包已认领");
      setLegacyUserHash("");
      setSignature("");
      setChallenge(null);
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "认领失败"),
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>旧钱包认领</CardTitle>
        <CardDescription>
          后端验证旧钱包对 challenge 的 Ed25519 签名；前端不接触旧钱包私钥或 PIN。
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={() => getChallenge.mutate()} disabled={getChallenge.isPending}>
            获取挑战
          </Button>
          {challenge ? (
            <div className="rounded-md border bg-muted px-3 py-2 text-xs">
              <p>challengeId: {challenge.challengeId}</p>
              <p>nonce: {challenge.nonce}</p>
            </div>
          ) : null}
        </div>
        <div className="grid gap-3 lg:grid-cols-2">
          <div className="space-y-2">
            <Label>legacyUserHash</Label>
            <Input value={legacyUserHash} onChange={(event) => setLegacyUserHash(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>旧钱包签名</Label>
            <Input value={signature} onChange={(event) => setSignature(event.target.value)} />
          </div>
        </div>
        <Button onClick={() => claim.mutate()} disabled={!challenge || !legacyUserHash || !signature || claim.isPending}>
          认领并合并余额
        </Button>
      </CardContent>
    </Card>
  );
}

function CreateTaskDialog() {
  const queryClient = useQueryClient();
  const [open, setOpen] = React.useState(false);
  const [title, setTitle] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [rewardAmount, setRewardAmount] = React.useState(10);
  const [contactInfo, setContactInfo] = React.useState("");
  const mutation = useMutation({
    mutationFn: () => {
      const body = { title, description: description || undefined, rewardAmount, contactInfo: contactInfo || undefined };
      return api.createTask(body, signIntent("create_task", body));
    },
    onSuccess: async () => {
      toast.success("悬赏已发布");
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["tasks"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "发布失败"),
  });

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus className="h-4 w-4" />
          发布悬赏
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>发布悬赏任务</DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-2">
            <Label>标题</Label>
            <Input value={title} onChange={(event) => setTitle(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>奖励积分</Label>
            <Input type="number" min={1} value={rewardAmount} onChange={(event) => setRewardAmount(Number(event.target.value))} />
          </div>
          <div className="space-y-2">
            <Label>描述</Label>
            <Textarea value={description} onChange={(event) => setDescription(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>接单后可见联系方式</Label>
            <Input value={contactInfo} onChange={(event) => setContactInfo(event.target.value)} />
          </div>
        </div>
        <DialogFooter>
          <Button onClick={() => mutation.mutate()} disabled={!title || rewardAmount <= 0 || mutation.isPending}>
            发布
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function CreateProductDialog() {
  const queryClient = useQueryClient();
  const [open, setOpen] = React.useState(false);
  const [title, setTitle] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [price, setPrice] = React.useState(10);
  const [stock, setStock] = React.useState(1);
  const [deliveryInfo, setDeliveryInfo] = React.useState("");
  const mutation = useMutation({
    mutationFn: () =>
      api.createProduct({
        title,
        description: description || undefined,
        price,
        stock,
        deliveryInfo: deliveryInfo || undefined,
      }),
    onSuccess: async () => {
      toast.success("商品已上架");
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["products"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "上架失败"),
  });

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline">
          <ShoppingBag className="h-4 w-4" />
          上架商品
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>上架商品</DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-2">
            <Label>标题</Label>
            <Input value={title} onChange={(event) => setTitle(event.target.value)} />
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>价格</Label>
              <Input type="number" min={1} value={price} onChange={(event) => setPrice(Number(event.target.value))} />
            </div>
            <div className="space-y-2">
              <Label>库存</Label>
              <Input type="number" min={0} value={stock} onChange={(event) => setStock(Number(event.target.value))} />
            </div>
          </div>
          <div className="space-y-2">
            <Label>描述</Label>
            <Textarea value={description} onChange={(event) => setDescription(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label>交付说明</Label>
            <Input value={deliveryInfo} onChange={(event) => setDeliveryInfo(event.target.value)} />
          </div>
        </div>
        <DialogFooter>
          <Button onClick={() => mutation.mutate()} disabled={!title || price <= 0 || mutation.isPending}>
            上架
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function TipPanel() {
  const queryClient = useQueryClient();
  const [toAccountId, setToAccountId] = React.useState("");
  const [amount, setAmount] = React.useState(1);
  const [targetType, setTargetType] = React.useState<"review" | "thread" | "comment">("thread");
  const [targetId, setTargetId] = React.useState("");
  const mutation = useMutation({
    mutationFn: () => {
      const body = { toAccountId, amount, targetType, targetId };
      return api.tip(body, signIntent("tip", body), `tip:${randomUuid()}`);
    },
    onSuccess: async () => {
      toast.success("打赏成功");
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => {
      const message = error instanceof Error ? error.message : "打赏失败";
      toast.error(`${message}。如果返回签名无效，需要后端先补签名前置 intent。`);
    },
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Gift className="h-4 w-4 text-primary" />
          内容打赏
        </CardTitle>
        <CardDescription>
          受控积分流转，必须绑定到 review/thread/comment；不提供自由转账。
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3 lg:grid-cols-[1fr_8rem_10rem_1fr_auto] lg:items-end">
        <div className="space-y-2">
          <Label>收款账号 ID</Label>
          <Input value={toAccountId} onChange={(event) => setToAccountId(event.target.value)} />
        </div>
        <div className="space-y-2">
          <Label>金额</Label>
          <Input type="number" min={1} value={amount} onChange={(event) => setAmount(Number(event.target.value))} />
        </div>
        <div className="space-y-2">
          <Label>内容类型</Label>
          <select
            className="h-9 w-full rounded-md border bg-transparent px-3 text-sm"
            value={targetType}
            onChange={(event) => setTargetType(event.target.value as "review" | "thread" | "comment")}
          >
            <option value="thread">thread</option>
            <option value="comment">comment</option>
            <option value="review">review</option>
          </select>
        </div>
        <div className="space-y-2">
          <Label>内容 ID</Label>
          <Input value={targetId} onChange={(event) => setTargetId(event.target.value)} />
        </div>
        <Button
          onClick={() => mutation.mutate()}
          disabled={!toAccountId || !targetId || amount <= 0 || mutation.isPending}
        >
          打赏
        </Button>
      </CardContent>
    </Card>
  );
}

function TaskCard({ task }: { task: Task }) {
  const queryClient = useQueryClient();
  const accept = useMutation({
    mutationFn: () => api.acceptTask(task.id ?? ""),
    onSuccess: async () => {
      toast.success("已接单");
      await queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "接单失败"),
  });
  const action = useMutation({
    mutationFn: (nextAction: "submit" | "confirm" | "cancel" | "reject" | "delete") =>
      api.taskAction(task.id ?? "", nextAction, signIntent("task_action", { id: task.id, action: nextAction })),
    onSuccess: async () => {
      toast.success("任务状态已更新");
      await queryClient.invalidateQueries({ queryKey: ["tasks"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="font-semibold">{task.title}</h3>
            <p className="mt-1 text-sm text-muted-foreground">{task.description ?? "无描述"}</p>
          </div>
          <Badge>{formatNumber(task.rewardAmount)} 积分</Badge>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Badge variant="secondary">{task.status}</Badge>
          <span>{formatDate(task.createdAt)}</span>
          {task.contactInfo ? <span>联系方式：{task.contactInfo}</span> : null}
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          {task.status === "open" ? <Button size="sm" variant="secondary" onClick={() => accept.mutate()}>接单</Button> : null}
          {task.status === "in_progress" ? <Button size="sm" variant="secondary" onClick={() => action.mutate("submit")}>提交完成</Button> : null}
          {task.status === "submitted" ? <Button size="sm" onClick={() => action.mutate("confirm")}>确认放款</Button> : null}
          {task.status !== "completed" && task.status !== "cancelled" ? (
            <Button size="sm" variant="outline" onClick={() => action.mutate("cancel")}>取消/拒绝</Button>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}

function ProductCard({ product }: { product: Product }) {
  const queryClient = useQueryClient();
  const buy = useMutation({
    mutationFn: () => api.purchaseProduct(product.id ?? "", signIntent("purchase_product", { id: product.id })),
    onSuccess: async () => {
      toast.success("已创建托管订单");
      await queryClient.invalidateQueries({ queryKey: ["products"] });
      await queryClient.invalidateQueries({ queryKey: ["purchases"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "购买失败"),
  });
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="font-semibold">{product.title}</h3>
            <p className="mt-1 text-sm text-muted-foreground">{product.description ?? "无描述"}</p>
          </div>
          <Badge>{formatNumber(product.price)} 积分</Badge>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Badge variant="secondary">{product.status}</Badge>
          <span>库存 {product.stock ?? 0}</span>
          <span>{formatDate(product.createdAt)}</span>
        </div>
        <Button size="sm" className="mt-3" onClick={() => buy.mutate()} disabled={buy.isPending || product.status !== "on_sale"}>
          购买并托管
        </Button>
      </CardContent>
    </Card>
  );
}

function PurchaseCard({ purchase }: { purchase: Purchase }) {
  const queryClient = useQueryClient();
  const action = useMutation({
    mutationFn: (nextAction: "accept" | "deliver" | "confirm") =>
      api.purchaseAction(purchase.id ?? "", nextAction, signIntent("purchase_action", { id: purchase.id, action: nextAction })),
    onSuccess: async () => {
      toast.success("订单状态已更新");
      await queryClient.invalidateQueries({ queryKey: ["purchases"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="font-semibold">订单 {purchase.id}</h3>
            <p className="mt-1 text-sm text-muted-foreground">商品 {purchase.productId}</p>
          </div>
          <Badge>{formatNumber(purchase.amount)} 积分</Badge>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Badge variant="secondary">{purchase.status}</Badge>
          {purchase.status === "pending" ? <Button size="sm" onClick={() => action.mutate("accept")}>卖家接单</Button> : null}
          {purchase.status === "accepted" ? <Button size="sm" onClick={() => action.mutate("deliver")}>标记交付</Button> : null}
          {purchase.status === "delivered" ? <Button size="sm" onClick={() => action.mutate("confirm")}>确认完成</Button> : null}
        </div>
      </CardContent>
    </Card>
  );
}

export function WalletPage() {
  const { isAuthenticated } = useAuth();
  const wallet = useQuery({ queryKey: ["wallet"], queryFn: api.wallet, enabled: isAuthenticated });
  const ledger = useQuery({ queryKey: ["ledger"], queryFn: () => api.ledger(), enabled: isAuthenticated });
  const verify = useQuery({ queryKey: ["ledger-verify"], queryFn: api.verifyLedger });
  const tasks = useQuery({ queryKey: ["tasks"], queryFn: () => api.tasks("all"), enabled: isAuthenticated });
  const products = useQuery({ queryKey: ["products"], queryFn: () => api.products(), enabled: isAuthenticated });
  const purchases = useQuery({ queryKey: ["purchases"], queryFn: () => api.purchases(), enabled: isAuthenticated });

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看积分钱包" description="积分是平台闭环权益，不支持充值、提现或自由转账。" />;
  }

  return (
    <div className="space-y-5">
      <PageHeader
        eyebrow="Credit"
        title="积分钱包"
        description="闭环 Web2.5 积分：贡献获得，平台内打赏、悬赏和商品托管使用。无充值、无提现、无自由转账。"
        actions={<><CreateTaskDialog /><CreateProductDialog /></>}
      />

      <div className="grid gap-4 lg:grid-cols-3">
        <Card>
          <CardContent className="flex items-center gap-4 p-5">
            <div className="rounded-md bg-secondary p-3 text-primary">
              <Wallet className="h-6 w-6" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">当前余额</p>
              <p className="text-3xl font-semibold">{wallet.isLoading ? "..." : formatNumber(wallet.data?.balance)}</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-4 p-5">
            <div className="rounded-md bg-secondary p-3 text-primary">
              <ShieldCheck className="h-6 w-6" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">账本校验</p>
              <p className="text-lg font-semibold">{verify.data?.ok ? "通过" : verify.isLoading ? "校验中" : "未通过/未知"}</p>
              <p className="text-xs text-muted-foreground">seq {verify.data?.latestSeq ?? 0} · {shortHash(verify.data?.latestHash)}</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-4 p-5">
            <div className="rounded-md bg-secondary p-3 text-primary">
              <CheckCircle2 className="h-6 w-6" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">合规边界</p>
              <p className="text-sm font-medium">无充值/提现/自由转账</p>
              <p className="text-xs text-muted-foreground">只在受控内容与 escrow 流程内流转</p>
            </div>
          </CardContent>
        </Card>
      </div>

      <WalletSetup />
      <LegacyClaimPanel />

      <TipPanel />

      <Tabs defaultValue="tasks">
        <TabsList className="w-full justify-start overflow-x-auto">
          <TabsTrigger value="tasks">悬赏任务</TabsTrigger>
          <TabsTrigger value="products">商品托管</TabsTrigger>
          <TabsTrigger value="purchases">我的订单</TabsTrigger>
          <TabsTrigger value="ledger">账本</TabsTrigger>
        </TabsList>

        <TabsContent value="tasks" className="space-y-3">
          {tasks.isLoading ? (
            <LoadingState />
          ) : tasks.isError ? (
            <ErrorState error={tasks.error} onRetry={() => void tasks.refetch()} />
          ) : (tasks.data?.items ?? []).length === 0 ? (
            <EmptyState title="暂无任务" />
          ) : (
            tasks.data?.items?.map((task) => <TaskCard key={task.id} task={task} />)
          )}
        </TabsContent>

        <TabsContent value="products" className="grid gap-3 md:grid-cols-2">
          {products.isLoading ? (
            <LoadingState />
          ) : products.isError ? (
            <ErrorState error={products.error} onRetry={() => void products.refetch()} />
          ) : (products.data?.items ?? []).length === 0 ? (
            <EmptyState title="暂无商品" className="md:col-span-2" />
          ) : (
            products.data?.items?.map((product) => <ProductCard key={product.id} product={product} />)
          )}
        </TabsContent>

        <TabsContent value="purchases" className="space-y-3">
          {purchases.isLoading ? (
            <LoadingState />
          ) : purchases.isError ? (
            <ErrorState error={purchases.error} onRetry={() => void purchases.refetch()} />
          ) : (purchases.data?.items ?? []).length === 0 ? (
            <EmptyState title="暂无订单" />
          ) : (
            purchases.data?.items?.map((purchase) => <PurchaseCard key={purchase.id} purchase={purchase} />)
          )}
        </TabsContent>

        <TabsContent value="ledger" className="space-y-2">
          {ledger.isLoading ? (
            <LoadingState />
          ) : ledger.isError ? (
            <ErrorState error={ledger.error} onRetry={() => void ledger.refetch()} />
          ) : (ledger.data?.items ?? []).length === 0 ? (
            <EmptyState title="暂无账本记录" />
          ) : (
            ledger.data?.items?.map((entry) => (
              <Card key={entry.seq}>
                <CardContent className="flex flex-col gap-2 p-4 md:flex-row md:items-center md:justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <Badge variant="secondary">{entry.type}</Badge>
                      <span className="font-medium">{formatNumber(entry.amount)} 积分</span>
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      #{entry.seq} · {formatDate(entry.createdAt)} · {shortHash(entry.hash)}
                    </p>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {entry.fromAccount ?? "system"} → {entry.toAccount ?? "escrow"}
                  </p>
                </CardContent>
              </Card>
            ))
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
