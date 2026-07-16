import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { CheckCircle2, Gift, KeyRound, Plus, ShieldCheck, ShoppingBag, Trash2, Wallet } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { RecentAuthDialog } from "@/components/auth/recent-auth-dialog";
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
import {
  readAccessToken,
  readAuthContextVersion,
  readStoredAccount,
} from "@/lib/auth-storage";
import { formatDate, formatNumber, shortHash } from "@/lib/format";
import {
  clearLocalWallet,
  createLocalWallet,
  discardLegacyWallet,
  getLocalWallet,
  inspectLegacyWallet,
  resolveWalletServerKeyState,
  type LocalWallet,
} from "@/lib/wallet";
import {
  performWalletMutation,
  reconcileWalletMutations,
  WalletMutationCommittedError,
  WalletMutationUncertainError,
} from "@/lib/wallet-mutations";

function refreshPendingWalletReconciliation(
  queryClient: QueryClient,
  accountId: string | undefined,
) {
  if (!accountId) return Promise.resolve();
  return queryClient.invalidateQueries({
    queryKey: ["wallet-pending-reconciliation", accountId],
    exact: true,
  });
}

function handleWalletMutationError(
  error: unknown,
  fallback: string,
  refreshCommittedState: () => Promise<void>,
  refreshPendingState: () => Promise<void>,
) {
  if (error instanceof WalletMutationCommittedError) {
    toast.success(error.message);
    void Promise.all([refreshCommittedState(), refreshPendingState()]);
    return;
  }
  toast.error(error instanceof Error ? error.message : fallback);
  if (error instanceof WalletMutationUncertainError) void refreshPendingState();
}

const I64_MAX = 9_223_372_036_854_775_807n;

interface PurchasableProduct {
  id: string;
  sellerId: string;
  price: number;
  stock: number;
  status: "on_sale";
}

function isCanonicalPositiveI64(value: unknown): value is string {
  if (typeof value !== "string" || !/^[1-9][0-9]*$/.test(value)) return false;
  try {
    return BigInt(value) <= I64_MAX;
  } catch {
    return false;
  }
}

function isPurchasableProduct(
  product: Product,
  buyerAccountId: string,
): product is Product & PurchasableProduct {
  return isCanonicalPositiveI64(product.id)
    && isCanonicalPositiveI64(product.sellerId)
    && product.sellerId !== buyerAccountId
    && Number.isSafeInteger(product.price)
    && product.price > 0
    && Number.isSafeInteger(product.stock)
    && product.stock > 0
    && product.status === "on_sale";
}

interface WalletBindingAuthorization {
  accountId: string;
  publicKey: string;
  authToken: string;
  contextVersion: number;
}

function captureWalletBindingAuthorization(accountId: string, publicKey: string) {
  const contextVersion = readAuthContextVersion();
  const authToken = readAccessToken();
  const storedAccountId = readStoredAccount()?.id;
  if (!authToken
    || storedAccountId !== accountId
    || readAuthContextVersion() !== contextVersion) {
    throw new Error("登录账号已变化，已停止绑定钱包");
  }
  return { accountId, publicKey, authToken, contextVersion };
}

function isWalletBindingAuthorizationCurrent(
  authorization: WalletBindingAuthorization,
  renderedAccountId: string | undefined,
) {
  const versionBeforeRead = readAuthContextVersion();
  const isCurrent = renderedAccountId === authorization.accountId
    && versionBeforeRead === authorization.contextVersion
    && readAccessToken() === authorization.authToken
    && readStoredAccount()?.id === authorization.accountId;
  return isCurrent && readAuthContextVersion() === versionBeforeRead;
}

function assertWalletBindingAuthorizationCurrent(
  authorization: WalletBindingAuthorization,
  renderedAccountId: string | undefined,
) {
  if (!isWalletBindingAuthorizationCurrent(authorization, renderedAccountId)) {
    throw new Error("登录账号已变化，已停止绑定钱包");
  }
}

export function WalletSetup({
  serverPublicKey,
  isServerStateKnown,
}: {
  serverPublicKey: string | null;
  isServerStateKnown: boolean;
}) {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const accountIdRef = React.useRef(account?.id);
  accountIdRef.current = account?.id;
  const [wallet, setWallet] = React.useState<LocalWallet | null>(null);
  const [legacyWallet, setLegacyWallet] = React.useState<LocalWallet | null>(null);
  const [loadedAccountId, setLoadedAccountId] = React.useState<string | null>(null);
  const [localError, setLocalError] = React.useState<Error | null>(null);
  const [isLocalWalletLoading, setIsLocalWalletLoading] = React.useState(true);
  const [reloadVersion, setReloadVersion] = React.useState(0);
  const [recentAuthOpen, setRecentAuthOpen] = React.useState(false);
  const [clearWalletOpen, setClearWalletOpen] = React.useState(false);
  const [discardLegacyOpen, setDiscardLegacyOpen] = React.useState(false);
  const [pendingBinding, setPendingBinding] = React.useState<{
    accountId: string;
    publicKey: string;
  } | null>(null);

  React.useEffect(() => {
    setRecentAuthOpen(false);
    setClearWalletOpen(false);
    setDiscardLegacyOpen(false);
    setPendingBinding(null);
  }, [account?.id]);

  React.useEffect(() => {
    let isCurrent = true;
    const accountId = account?.id;
    if (!accountId || !isServerStateKnown) {
      setLoadedAccountId(accountId ?? null);
      setWallet(null);
      setLegacyWallet(null);
      setLocalError(null);
      setIsLocalWalletLoading(!isServerStateKnown);
      return () => {
        isCurrent = false;
      };
    }
    setLoadedAccountId(accountId);
    setWallet(null);
    setLegacyWallet(null);
    setLocalError(null);
    setIsLocalWalletLoading(true);
    void getLocalWallet(accountId, serverPublicKey)
      .then((nextWallet) => {
        if (!isCurrent) return;
        setWallet(nextWallet);
        setLegacyWallet(inspectLegacyWallet());
        setLocalError(null);
      })
      .catch((error: unknown) => {
        if (!isCurrent) return;
        setWallet(null);
        setLocalError(error instanceof Error ? error : new Error("无法读取本机钱包"));
        try {
          setLegacyWallet(inspectLegacyWallet());
        } catch {
          setLegacyWallet(null);
        }
      })
      .finally(() => {
        if (isCurrent) setIsLocalWalletLoading(false);
      });
    return () => {
      isCurrent = false;
    };
  }, [account?.id, isServerStateKnown, reloadVersion, serverPublicKey]);

  const bind = useMutation({
    mutationFn: async (authorization: WalletBindingAuthorization) => {
      assertWalletBindingAuthorizationCurrent(authorization, accountIdRef.current);
      const ownerWallet = await api.wallet(authorization.authToken);
      assertWalletBindingAuthorizationCurrent(authorization, accountIdRef.current);
      const serverKeyState = resolveWalletServerKeyState(ownerWallet, authorization.accountId);
      if (!serverKeyState.isKnown
        || (serverKeyState.activePublicKey !== null
          && serverKeyState.activePublicKey !== authorization.publicKey)) {
        throw new Error("服务端钱包公钥状态已变化，已停止绑定钱包");
      }
      await api.bindWallet(
        authorization.accountId,
        authorization.publicKey,
        authorization.authToken,
      );
      assertWalletBindingAuthorizationCurrent(authorization, accountIdRef.current);
      return authorization.accountId;
    },
    onSuccess: async (boundAccountId) => {
      if (accountIdRef.current !== boundAccountId) return;
      setPendingBinding(null);
      toast.success("钱包公钥已绑定");
      await queryClient.refetchQueries({ queryKey: ["wallet"] });
      setReloadVersion((version) => version + 1);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "绑定失败"),
  });
  const create = useMutation({
    mutationFn: async (accountId: string) => {
      if (accountIdRef.current !== accountId || !isServerStateKnown) {
        throw new Error("账号已切换，已停止生成钱包");
      }
      const nextWallet = await createLocalWallet(accountId, serverPublicKey);
      return { accountId, wallet: nextWallet };
    },
    onSuccess: (result) => {
      if (accountIdRef.current !== result.accountId) return;
      setLoadedAccountId(result.accountId);
      setWallet(result.wallet);
      setLocalError(null);
      try {
        setLegacyWallet(inspectLegacyWallet());
      } catch {
        setLegacyWallet(null);
      }
      toast.success("已生成不可导出的本地钱包密钥");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "生成钱包失败"),
  });
  const clear = useMutation({
    mutationFn: async (accountId: string) => {
      if (accountIdRef.current !== accountId) throw new Error("账号已切换，已停止清除钱包");
      await clearLocalWallet(accountId);
      return accountId;
    },
    onSuccess: (clearedAccountId) => {
      if (accountIdRef.current !== clearedAccountId) return;
      setWallet(null);
      setLocalError(null);
      setClearWalletOpen(false);
      setReloadVersion((version) => version + 1);
      toast.success("已清除当前环境与账号的本机私钥");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "清除钱包失败"),
  });
  const discardLegacy = useMutation({
    mutationFn: async (accountId: string) => {
      if (accountIdRef.current !== accountId) throw new Error("账号已切换，已停止丢弃旧钱包");
      discardLegacyWallet();
      return accountId;
    },
    onSuccess: (discardedForAccountId) => {
      if (accountIdRef.current !== discardedForAccountId) return;
      setLegacyWallet(null);
      setDiscardLegacyOpen(false);
      setReloadVersion((version) => version + 1);
      toast.success("已丢弃旧版浏览器钱包私钥");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "丢弃旧钱包失败"),
  });

  const scopedWallet = loadedAccountId === account?.id ? wallet : null;
  const scopedLegacyWallet = loadedAccountId === account?.id ? legacyWallet : null;
  const isBound = Boolean(scopedWallet && serverPublicKey === scopedWallet.publicKey);
  const bindCandidate = isServerStateKnown && serverPublicKey === null
    ? scopedWallet ?? scopedLegacyWallet
    : null;

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="h-4 w-4 text-primary" />
            本地签名钱包
          </CardTitle>
          <CardDescription>
            私钥以不可导出的 WebCrypto 密钥保存，并按 API 环境和账号隔离；平台只保存验证公钥。
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {isLocalWalletLoading ? <LoadingState label="读取本机钱包" /> : null}
          {!isLocalWalletLoading && scopedWallet ? (
            <div className="rounded-md border bg-muted/60 p-3 text-xs">
              <div className="flex items-center justify-between gap-2">
                <p className="font-medium">本机公钥</p>
                {isBound ? <Badge variant="secondary">已与服务端匹配</Badge> : null}
              </div>
              <p className="mt-1 break-all text-muted-foreground">{scopedWallet.publicKey}</p>
            </div>
          ) : null}
          {!isLocalWalletLoading && !scopedWallet && serverPublicKey ? (
            <p className="rounded-md border border-destructive/40 p-3 text-sm text-destructive" role="status">
              服务端已有钱包公钥，但当前环境与账号没有匹配的本机私钥。系统不会生成替代密钥或仅凭登录态换绑。
            </p>
          ) : null}
          {!isLocalWalletLoading && !scopedWallet && !serverPublicKey && !scopedLegacyWallet ? (
            <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">
              当前环境与账号还没有钱包密钥。
            </p>
          ) : null}
          {localError ? (
            <p className="rounded-md border border-destructive/40 p-3 text-sm text-destructive" role="status">
              {localError.message}
            </p>
          ) : null}
          {scopedLegacyWallet ? (
            <div className="rounded-md border border-amber-500/40 bg-amber-500/5 p-3 text-sm">
              <p className="font-medium">检测到旧版浏览器钱包</p>
              <p className="mt-1 break-all text-xs text-muted-foreground">{scopedLegacyWallet.publicKey}</p>
              <p className="mt-2 text-xs text-muted-foreground">
                {serverPublicKey === null
                  ? "只有你明确绑定该公钥后，系统才会把旧 seed 迁移为不可导出的账号密钥。"
                  : "它未自动归属当前账号；请切回匹配账号迁移，或确认不再需要后再丢弃。"}
              </p>
            </div>
          ) : null}
          <div className="flex flex-wrap gap-2">
            {!scopedWallet && !scopedLegacyWallet && serverPublicKey === null && isServerStateKnown ? (
              <Button
                variant="secondary"
                onClick={() => {
                  if (account?.id) create.mutate(account.id);
                }}
                disabled={create.isPending}
              >
                生成本地钱包
              </Button>
            ) : null}
            {bindCandidate ? (
              <Button
                onClick={() => {
                  const accountId = accountIdRef.current;
                  if (!accountId) return;
                  setPendingBinding({ accountId, publicKey: bindCandidate.publicKey });
                  setRecentAuthOpen(true);
                }}
                disabled={bind.isPending}
              >
                {!scopedWallet && scopedLegacyWallet ? "绑定并迁移旧钱包" : "绑定公钥"}
              </Button>
            ) : null}
            {scopedWallet ? (
              <Button
                variant="outline"
                onClick={() => setClearWalletOpen(true)}
                disabled={clear.isPending}
              >
                <Trash2 className="h-4 w-4" />
                清除本机私钥
              </Button>
            ) : null}
            {scopedLegacyWallet ? (
              <Button
                variant="outline"
                onClick={() => setDiscardLegacyOpen(true)}
                disabled={discardLegacy.isPending}
              >
                丢弃旧版私钥
              </Button>
            ) : null}
          </div>
        </CardContent>
      </Card>
      <RecentAuthDialog
        open={recentAuthOpen}
        onOpenChange={setRecentAuthOpen}
        description="首次绑定钱包公钥会授权本机签署积分操作，需要当前会话在最近 10 分钟内重新验证。已有公钥不能仅凭登录态轮换。"
        onVerified={() => {
          if (pendingBinding && accountIdRef.current === pendingBinding.accountId) {
            try {
              bind.mutate(captureWalletBindingAuthorization(
                pendingBinding.accountId,
                pendingBinding.publicKey,
              ));
            } catch (error) {
              toast.error(error instanceof Error ? error.message : "绑定失败");
            }
          }
        }}
      />
      <Dialog open={clearWalletOpen} onOpenChange={setClearWalletOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>清除本机钱包私钥？</DialogTitle>
          </DialogHeader>
          <p className="text-sm leading-6 text-muted-foreground">
            服务端绑定的公钥不会随本机数据清除。清除后，本机将不能再签署积分操作；当前尚没有仅凭登录态恢复或轮换钱包密钥的入口。
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setClearWalletOpen(false)}>取消</Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (loadedAccountId) clear.mutate(loadedAccountId);
              }}
              disabled={clear.isPending}
            >
              确认清除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog open={discardLegacyOpen} onOpenChange={setDiscardLegacyOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>永久丢弃旧版浏览器私钥？</DialogTitle>
          </DialogHeader>
          <p className="text-sm leading-6 text-muted-foreground">
            该操作只删除旧版全局 localStorage seed，无法恢复，也不会修改服务端已绑定的公钥。请先确认它不属于你仍需使用的其他账号。
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDiscardLegacyOpen(false)}>取消</Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (loadedAccountId) discardLegacy.mutate(loadedAccountId);
              }}
              disabled={discardLegacy.isPending}
            >
              确认永久丢弃
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
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
      setSignature("");
      toast.success("认领挑战已生成，10 分钟内有效");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "获取挑战失败"),
  });
  const claim = useMutation({
    mutationFn: () => {
      if (!challenge?.challengeId) {
        throw new Error("请先获取挑战");
      }
      return api.claimWallet({
        legacyUserHash: legacyUserHash.trim(),
        challengeId: challenge.challengeId,
        signature: signature.trim(),
      });
    },
    onSuccess: async () => {
      toast.success("旧钱包已认领");
      setLegacyUserHash("");
      setSignature("");
      setChallenge(null);
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: () => {
      setChallenge(null);
      setSignature("");
      toast.error("认领未完成；本次挑战已失效，请重新获取并签名");
    },
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>旧钱包认领</CardTitle>
        <CardDescription>
          通过旧钱包签名验证身份后合并余额；平台不会接触你的旧钱包私钥或 PIN。
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
            <Label htmlFor="legacy-wallet-user-hash">legacyUserHash</Label>
            <Input
              id="legacy-wallet-user-hash"
              value={legacyUserHash}
              maxLength={64}
              autoCapitalize="none"
              spellCheck={false}
              onChange={(event) => setLegacyUserHash(event.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="legacy-wallet-signature">旧钱包签名</Label>
            <Input
              id="legacy-wallet-signature"
              value={signature}
              maxLength={88}
              autoCapitalize="none"
              spellCheck={false}
              onChange={(event) => setSignature(event.target.value)}
            />
          </div>
        </div>
        <Button
          onClick={() => claim.mutate()}
          disabled={
            !challenge
            || !/^[0-9a-f]{64}$/.test(legacyUserHash.trim())
            || !/^[A-Za-z0-9+/]{86}==$/.test(signature.trim())
            || claim.isPending
          }
        >
          认领并合并余额
        </Button>
      </CardContent>
    </Card>
  );
}

function CreateTaskDialog() {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const [open, setOpen] = React.useState(false);
  const [title, setTitle] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [rewardAmount, setRewardAmount] = React.useState(10);
  const [contactInfo, setContactInfo] = React.useState("");
  const mutation = useMutation({
    mutationFn: async () => {
      if (!account?.id) throw new Error("登录账号尚未就绪");
      const body = { title, description: description || undefined, rewardAmount, contactInfo: contactInfo || undefined };
      return performWalletMutation(
        account.id,
        "credit.task.create",
        body,
        { kind: "taskCreate" },
        (authorization, authToken) => api.createTask(body, authorization, authToken),
      );
    },
    onSuccess: async () => {
      toast.success("悬赏已发布");
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["tasks"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => handleWalletMutationError(
      error,
      "发布失败",
      async () => {
        setOpen(false);
        await queryClient.invalidateQueries({ queryKey: ["tasks"] });
        await queryClient.invalidateQueries({ queryKey: ["wallet"] });
        await queryClient.invalidateQueries({ queryKey: ["ledger"] });
      },
      () => refreshPendingWalletReconciliation(queryClient, account?.id),
    ),
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
          <Button onClick={() => mutation.mutate()} disabled={!title || price <= 0 || stock < 0 || mutation.isPending}>
            上架
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function TipPanel() {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const [toAccountId, setToAccountId] = React.useState("");
  const [amount, setAmount] = React.useState(1);
  const [targetType, setTargetType] = React.useState<"review" | "thread" | "comment">("thread");
  const [targetId, setTargetId] = React.useState("");
  const mutation = useMutation({
    mutationFn: async () => {
      if (!account?.id) throw new Error("登录账号尚未就绪");
      const body = { toAccountId, amount, targetType, targetId };
      return performWalletMutation(
        account.id,
        "credit.tip",
        body,
        { kind: "tip" },
        (authorization, authToken) => api.tip(body, authorization, authToken),
      );
    },
    onSuccess: async () => {
      toast.success("打赏成功");
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => handleWalletMutationError(
      error,
      "打赏失败",
      async () => {
        await queryClient.invalidateQueries({ queryKey: ["wallet"] });
        await queryClient.invalidateQueries({ queryKey: ["ledger"] });
      },
      () => refreshPendingWalletReconciliation(queryClient, account?.id),
    ),
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
  const { account } = useAuth();
  const isCreator = account?.id === task.creatorId;
  const isAcceptor = account?.id === task.acceptorId;
  const accept = useMutation({
    mutationFn: () => api.acceptTask(task.id ?? ""),
    onSuccess: async () => {
      toast.success("已接单");
      await queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "接单失败"),
  });
  const action = useMutation({
    mutationFn: async (nextAction: "submit" | "confirm" | "cancel" | "reject" | "delete") => {
      if (nextAction === "submit" || (nextAction === "delete" && task.status === "cancelled")) {
        return api.taskAction(task.id ?? "", nextAction);
      }
      if (!account?.id) throw new Error("登录账号尚未就绪");
      const request = { id: task.id ?? "", action: nextAction };
      return performWalletMutation(
        account.id,
        "credit.task.action",
        request,
        {
          kind: "taskAction",
          task: {
            id: task.id ?? "",
            creatorId: task.creatorId ?? "",
            acceptorId: task.acceptorId ?? null,
            rewardAmount: task.rewardAmount ?? Number.NaN,
            status: task.status ?? "",
          },
        },
        (authorization, authToken) => (
          api.taskAction(task.id ?? "", nextAction, authorization, authToken)
        ),
      );
    },
    onSuccess: async () => {
      toast.success("任务状态已更新");
      await queryClient.invalidateQueries({ queryKey: ["tasks"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => handleWalletMutationError(
      error,
      "操作失败",
      async () => {
        await queryClient.invalidateQueries({ queryKey: ["tasks"] });
        await queryClient.invalidateQueries({ queryKey: ["wallet"] });
        await queryClient.invalidateQueries({ queryKey: ["ledger"] });
      },
      () => refreshPendingWalletReconciliation(queryClient, account?.id),
    ),
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
          {task.status === "open" && !isCreator ? <Button size="sm" variant="secondary" disabled={accept.isPending} onClick={() => accept.mutate()}>接单</Button> : null}
          {task.status === "in_progress" && isAcceptor ? <Button size="sm" variant="secondary" disabled={action.isPending} onClick={() => action.mutate("submit")}>提交完成</Button> : null}
          {task.status === "submitted" && isCreator ? <Button size="sm" disabled={action.isPending} onClick={() => action.mutate("confirm")}>确认放款</Button> : null}
          {isCreator && task.status !== "completed" && task.status !== "cancelled" ? (
            <Button size="sm" variant="outline" disabled={action.isPending} onClick={() => action.mutate("cancel")}>取消并退款</Button>
          ) : null}
          {isAcceptor && (task.status === "in_progress" || task.status === "submitted") ? (
            <Button size="sm" variant="outline" disabled={action.isPending} onClick={() => action.mutate("reject")}>拒绝并退款</Button>
          ) : null}
          {isCreator && (task.status === "open" || task.status === "cancelled") ? (
            <Button size="sm" variant="ghost" disabled={action.isPending} onClick={() => action.mutate("delete")}>删除任务</Button>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}

function ProductCard({ product }: { product: Product }) {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const buy = useMutation({
    mutationFn: async () => {
      if (!account?.id) throw new Error("登录账号尚未就绪");
      if (!isPurchasableProduct(product, account.id)) {
        throw new Error("页面展示的商品信息不完整或不可购买");
      }
      const request = { productId: product.id };
      return performWalletMutation(
        account.id,
        "credit.product.purchase",
        request,
        {
          kind: "productPurchase",
          product: {
            id: product.id,
            price: product.price,
            sellerId: product.sellerId,
            status: product.status,
            stock: product.stock,
          },
        },
        (authorization, authToken) => (
          api.purchaseProduct(product.id, authorization, authToken)
        ),
      );
    },
    onSuccess: async () => {
      toast.success("已创建托管订单");
      await queryClient.invalidateQueries({ queryKey: ["products"] });
      await queryClient.invalidateQueries({ queryKey: ["purchases"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => handleWalletMutationError(
      error,
      "购买失败",
      async () => {
        await queryClient.invalidateQueries({ queryKey: ["products"] });
        await queryClient.invalidateQueries({ queryKey: ["purchases"] });
        await queryClient.invalidateQueries({ queryKey: ["wallet"] });
        await queryClient.invalidateQueries({ queryKey: ["ledger"] });
      },
      () => refreshPendingWalletReconciliation(queryClient, account?.id),
    ),
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
        <Button
          size="sm"
          className="mt-3"
          onClick={() => buy.mutate()}
          disabled={buy.isPending || !account?.id || !isPurchasableProduct(product, account.id)}
        >
          购买并托管
        </Button>
      </CardContent>
    </Card>
  );
}

function PurchaseCard({ purchase }: { purchase: Purchase }) {
  const queryClient = useQueryClient();
  const { account } = useAuth();
  const isBuyer = account?.id === purchase.buyerId;
  const isSeller = account?.id === purchase.sellerId;
  const action = useMutation({
    mutationFn: async (nextAction: "accept" | "deliver" | "confirm" | "cancel") => {
      if (nextAction === "accept" || nextAction === "deliver") {
        return api.purchaseAction(purchase.id ?? "", nextAction);
      }
      if (!account?.id) throw new Error("登录账号尚未就绪");
      const request = { id: purchase.id ?? "", action: nextAction };
      return performWalletMutation(
        account.id,
        "credit.purchase.action",
        request,
        {
          kind: "purchaseAction",
          purchase: {
            id: purchase.id ?? "",
            buyerId: purchase.buyerId ?? "",
            sellerId: purchase.sellerId ?? "",
            amount: purchase.amount ?? Number.NaN,
            status: purchase.status ?? "",
          },
        },
        (authorization, authToken) => (
          api.purchaseAction(purchase.id ?? "", nextAction, authorization, authToken)
        ),
      );
    },
    onSuccess: async () => {
      toast.success("订单状态已更新");
      await queryClient.invalidateQueries({ queryKey: ["purchases"] });
      await queryClient.invalidateQueries({ queryKey: ["wallet"] });
      await queryClient.invalidateQueries({ queryKey: ["ledger"] });
    },
    onError: (error) => handleWalletMutationError(
      error,
      "操作失败",
      async () => {
        await queryClient.invalidateQueries({ queryKey: ["purchases"] });
        await queryClient.invalidateQueries({ queryKey: ["wallet"] });
        await queryClient.invalidateQueries({ queryKey: ["ledger"] });
      },
      () => refreshPendingWalletReconciliation(queryClient, account?.id),
    ),
  });
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="font-semibold">订单 {purchase.id}</h3>
            <p className="mt-1 text-sm text-muted-foreground">商品 {purchase.productId}</p>
            {purchase.deliveryInfo ? <p className="mt-1 text-sm">交付说明：{purchase.deliveryInfo}</p> : null}
          </div>
          <Badge>{formatNumber(purchase.amount)} 积分</Badge>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Badge variant="secondary">{purchase.status}</Badge>
          <span className="text-xs text-muted-foreground">{formatDate(purchase.createdAt)}</span>
          {isSeller && purchase.status === "pending" ? <Button size="sm" disabled={action.isPending} onClick={() => action.mutate("accept")}>卖家接单</Button> : null}
          {isSeller && purchase.status === "accepted" ? <Button size="sm" disabled={action.isPending} onClick={() => action.mutate("deliver")}>标记交付</Button> : null}
          {isBuyer && purchase.status === "delivered" ? <Button size="sm" disabled={action.isPending} onClick={() => action.mutate("confirm")}>确认完成</Button> : null}
          {isBuyer && (purchase.status === "pending" || purchase.status === "accepted") ? (
            <Button size="sm" variant="outline" disabled={action.isPending} onClick={() => action.mutate("cancel")}>取消并退款</Button>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}

export function WalletPage() {
  const queryClient = useQueryClient();
  const { isAuthenticated, account } = useAuth();
  const wallet = useQuery({
    queryKey: ["wallet"],
    queryFn: () => api.wallet(),
    enabled: isAuthenticated,
  });
  const ledger = useQuery({ queryKey: ["ledger"], queryFn: () => api.ledger(), enabled: isAuthenticated });
  const verify = useQuery({ queryKey: ["ledger-verify"], queryFn: api.verifyLedger });
  const tasks = useQuery({ queryKey: ["tasks"], queryFn: () => api.tasks("all"), enabled: isAuthenticated });
  const products = useQuery({ queryKey: ["products"], queryFn: () => api.products(), enabled: isAuthenticated });
  const purchases = useQuery({ queryKey: ["purchases"], queryFn: () => api.purchases(), enabled: isAuthenticated });
  const serverKeyState = resolveWalletServerKeyState(wallet.data, account?.id ?? null);
  const isServerStateKnown = wallet.isSuccess && serverKeyState.isKnown;
  const pendingReconciliation = useQuery({
    queryKey: ["wallet-pending-reconciliation", account?.id],
    queryFn: () => {
      if (!account?.id) throw new Error("登录账号尚未就绪");
      return reconcileWalletMutations(account.id);
    },
    enabled: isAuthenticated && Boolean(account?.id),
    retry: false,
    staleTime: 0,
    refetchOnMount: "always",
  });

  React.useEffect(() => {
    if (!pendingReconciliation.data?.resolvedCount) return;
    toast.success("已根据服务端状态核验并清理待确认的积分操作");
    void Promise.all([
      queryClient.invalidateQueries({ queryKey: ["wallet"] }),
      queryClient.invalidateQueries({ queryKey: ["ledger"] }),
      queryClient.invalidateQueries({ queryKey: ["tasks"] }),
      queryClient.invalidateQueries({ queryKey: ["products"] }),
      queryClient.invalidateQueries({ queryKey: ["purchases"] }),
    ]);
  }, [pendingReconciliation.data?.resolvedCount, queryClient]);

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看积分钱包" description="积分是平台闭环权益，不支持充值、提现或自由转账。" />;
  }

  return (
    <div className="space-y-5">
      <PageHeader
        title="积分钱包"
        description="通过贡献获得积分，可在平台内打赏、悬赏和兑换使用。不支持充值、提现或自由转账。"
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

      {wallet.isError || (wallet.isSuccess && !isServerStateKnown) ? (
        <Card className="border-destructive/40">
          <CardContent className="p-4 text-sm text-destructive" role="status">
            无法确认当前账号的服务端钱包公钥；可能是后端版本尚未完成切换或响应不完整。为避免错签，钱包密钥操作与积分写入已停止。
          </CardContent>
        </Card>
      ) : null}

      {pendingReconciliation.isError || (pendingReconciliation.data?.unresolvedCount ?? 0) > 0 ? (
        <Card className="border-amber-500/40 bg-amber-500/5">
          <CardContent className="p-4 text-sm">
            <p className="font-medium">存在尚未完成核验的积分操作</p>
            <p className="mt-1 text-muted-foreground">
              {pendingReconciliation.isError
                ? "当前浏览器无法读取或核验本地记录。为避免重复扣款，相关操作会保持关闭，刷新后可重试。"
                : `仍有 ${pendingReconciliation.data?.unresolvedCount ?? 0} 条记录无法从服务端确认；相同操作暂时不会重复发送。`}
            </p>
          </CardContent>
        </Card>
      ) : null}

      <WalletSetup
        serverPublicKey={serverKeyState.activePublicKey}
        isServerStateKnown={isServerStateKnown}
      />
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
