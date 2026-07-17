import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/design/app_theme.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../../settings/presentation/recent_auth_dialog.dart';
import '../data/wallet_repository.dart';
import 'wallet_forms.dart';
import 'wallet_records.dart';

final Provider<WalletRepository> walletRepositoryProvider =
    Provider<WalletRepository>((Ref ref) {
      final YourtjApi api = ref.watch(apiProvider);
      final WalletRepository repository = WalletRepository(
        api.getWalletApi(),
        api.getCreditApi(),
        ref.watch(sessionManagerProvider),
        ref.watch(walletSignerProvider),
        pendingMutationStore: ref.watch(walletPendingMutationStoreProvider),
      );
      return repository;
    });

class WalletPage extends ConsumerStatefulWidget {
  const WalletPage({super.key});

  @override
  ConsumerState<WalletPage> createState() => _WalletPageState();
}

class _WalletPageState extends ConsumerState<WalletPage> {
  Future<WalletSnapshot>? _snapshot;
  String? _loadedAccountId;
  bool _isMutating = false;

  Future<WalletSnapshot> _load(String accountId) {
    if (_loadedAccountId != accountId || _snapshot == null) {
      _loadedAccountId = accountId;
      _snapshot = ref.read(walletRepositoryProvider).load();
    }
    return _snapshot!;
  }

  void _reload() {
    if (_loadedAccountId == null) {
      return;
    }
    setState(() {
      _snapshot = ref.read(walletRepositoryProvider).load();
    });
  }

  Future<void> _mutate(
    Future<void> Function(WalletRepository repository) action,
    String successMessage,
  ) async {
    if (_isMutating) {
      return;
    }
    setState(() => _isMutating = true);
    try {
      await action(ref.read(walletRepositoryProvider));
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(successMessage)));
      _reload();
    } on WalletMutationCommitted {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('操作已经提交，已从权威状态核验成功')));
        _reload();
      }
    } on WalletMutationUncertain catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(failure.toString()),
            duration: const Duration(seconds: 8),
            action: SnackBarAction(label: '立即核验', onPressed: _reload),
          ),
        );
        _reload();
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    return session.when(
      loading: () => const AppLoadingState(title: '正在恢复积分钱包'),
      error: (Object error, StackTrace stackTrace) => AppErrorState(
        title: '无法读取登录状态',
        onRetry: () => ref.invalidate(sessionStateProvider),
      ),
      data: (SessionState state) {
        if (!state.isAuthenticated || state.account == null) {
          return AppEmptyState(
            title: '登录后查看积分钱包',
            description: '积分是平台闭环权益，不支持充值、提现或自由转账。',
            action: FilledButton.icon(
              onPressed: () => context.push(AppRoutes.login),
              icon: const Icon(Icons.login_rounded),
              label: const Text('登录或注册'),
            ),
          );
        }
        return FutureBuilder<WalletSnapshot>(
          future: _load(state.account!.id),
          builder:
              (BuildContext context, AsyncSnapshot<WalletSnapshot> snapshot) {
                if (snapshot.connectionState != ConnectionState.done) {
                  return const AppLoadingState(title: '正在读取积分钱包');
                }
                if (snapshot.hasError) {
                  final Object error = snapshot.error!;
                  return AppErrorState(
                    title: '积分钱包加载失败',
                    description: error is ApiFailure
                        ? error.message
                        : '暂时无法读取钱包，请稍后重试。',
                    onRetry: _reload,
                  );
                }
                return _WalletContent(
                  snapshot: snapshot.requireData,
                  accountId: state.account!.id,
                  isMutating: _isMutating,
                  onRefresh: () async => _reload(),
                  onCreateTask: () async {
                    final TaskInput? input = await showCreateTaskDialog(
                      context,
                    );
                    if (input == null) {
                      return;
                    }
                    await _mutate((WalletRepository repository) async {
                      await repository.createTask(input);
                    }, '悬赏已发布并进入托管流程');
                  },
                  onCreateProduct: () async {
                    final ProductInput? input = await showCreateProductDialog(
                      context,
                    );
                    if (input == null) {
                      return;
                    }
                    await _mutate((WalletRepository repository) async {
                      await repository.createProduct(input);
                    }, '商品已上架');
                  },
                  onBindWallet: () async {
                    if (!await ensureRecentAuthentication(context, ref)) {
                      return;
                    }
                    await _mutate((WalletRepository repository) async {
                      await repository.createAndBindLocalKey();
                    }, '本机钱包公钥已绑定');
                  },
                  onDeleteWallet: () async {
                    final bool confirmed = await _confirmWalletDeletion(
                      context,
                    );
                    if (!confirmed) {
                      return;
                    }
                    await _mutate(
                      (WalletRepository repository) =>
                          repository.deleteLocalKey(),
                      '已清除本机钱包私钥',
                    );
                  },
                  onClaimWallet: () async {
                    final bool claimed = await showLegacyClaimDialog(
                      context,
                      ref.read(walletRepositoryProvider),
                    );
                    if (claimed && mounted) {
                      _reload();
                    }
                  },
                  onTip: (TipInput input) => _mutate(
                    (WalletRepository repository) => repository.tip(input),
                    '打赏成功',
                  ),
                  onAcceptTask: (String taskId) => _mutate(
                    (WalletRepository repository) =>
                        repository.acceptTask(taskId),
                    '已接单',
                  ),
                  onTaskAction: (Task task, TaskActionActionEnum action) =>
                      _mutate(
                        (WalletRepository repository) =>
                            repository.updateTask(task: task, action: action),
                        '任务状态已更新',
                      ),
                  onPurchase: (Product product) =>
                      _mutate((WalletRepository repository) async {
                        await repository.purchaseProduct(product);
                      }, '已创建托管订单'),
                  onPurchaseAction:
                      (Purchase purchase, PurchaseActionActionEnum action) =>
                          _mutate(
                            (WalletRepository repository) =>
                                repository.updatePurchase(
                                  purchase: purchase,
                                  action: action,
                                ),
                            '订单状态已更新',
                          ),
                );
              },
        );
      },
    );
  }
}

class _WalletContent extends StatelessWidget {
  const _WalletContent({
    required this.snapshot,
    required this.accountId,
    required this.isMutating,
    required this.onRefresh,
    required this.onCreateTask,
    required this.onCreateProduct,
    required this.onBindWallet,
    required this.onDeleteWallet,
    required this.onClaimWallet,
    required this.onTip,
    required this.onAcceptTask,
    required this.onTaskAction,
    required this.onPurchase,
    required this.onPurchaseAction,
  });

  final WalletSnapshot snapshot;
  final String accountId;
  final bool isMutating;
  final Future<void> Function() onRefresh;
  final VoidCallback onCreateTask;
  final VoidCallback onCreateProduct;
  final VoidCallback onBindWallet;
  final VoidCallback onDeleteWallet;
  final VoidCallback onClaimWallet;
  final ValueChanged<TipInput> onTip;
  final ValueChanged<String> onAcceptTask;
  final void Function(Task task, TaskActionActionEnum action) onTaskAction;
  final ValueChanged<Product> onPurchase;
  final void Function(Purchase purchase, PurchaseActionActionEnum action)
  onPurchaseAction;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: RefreshIndicator(
        onRefresh: onRefresh,
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.all(16),
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 1120),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  _WalletHeader(
                    isMutating: isMutating,
                    onCreateTask: onCreateTask,
                    onCreateProduct: onCreateProduct,
                  ),
                  const SizedBox(height: 16),
                  _WalletSummary(snapshot: snapshot),
                  if (snapshot.pendingMutationCount > 0) ...<Widget>[
                    const SizedBox(height: 16),
                    Card(
                      color: Theme.of(context).colorScheme.tertiaryContainer,
                      child: const ListTile(
                        leading: Icon(Icons.sync_problem_rounded),
                        title: Text('有积分操作仍在等待权威状态确认'),
                        subtitle: Text(
                          '客户端已安全保留操作标识并阻止重复签名。请下拉刷新；确认前不要再次发起同一操作。',
                        ),
                      ),
                    ),
                  ],
                  const SizedBox(height: 16),
                  _WalletKeyPanel(
                    publicKey: snapshot.localKey?.publicKeyBase64,
                    isMutating: isMutating,
                    onBind: onBindWallet,
                    onDelete: onDeleteWallet,
                    onClaim: onClaimWallet,
                  ),
                  const SizedBox(height: 16),
                  WalletTipComposer(isSubmitting: isMutating, onSubmit: onTip),
                  const SizedBox(height: 20),
                  WalletRecordTabs(
                    snapshot: snapshot,
                    accountId: accountId,
                    isMutating: isMutating,
                    onAcceptTask: onAcceptTask,
                    onTaskAction: onTaskAction,
                    onPurchase: onPurchase,
                    onPurchaseAction: onPurchaseAction,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _WalletHeader extends StatelessWidget {
  const _WalletHeader({
    required this.isMutating,
    required this.onCreateTask,
    required this.onCreateProduct,
  });

  final bool isMutating;
  final VoidCallback onCreateTask;
  final VoidCallback onCreateProduct;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 12,
      runSpacing: 12,
      crossAxisAlignment: WrapCrossAlignment.center,
      alignment: WrapAlignment.spaceBetween,
      children: <Widget>[
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 680),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Semantics(
                header: true,
                child: Text(
                  '积分钱包',
                  style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(height: 6),
              const Text('通过贡献获得积分，在平台受控的打赏、悬赏和兑换流程内使用；不支持充值、提现或自由转账。'),
            ],
          ),
        ),
        Wrap(
          spacing: 8,
          children: <Widget>[
            FilledButton.icon(
              onPressed: isMutating ? null : onCreateTask,
              icon: const Icon(Icons.add_task_rounded),
              label: const Text('发布悬赏'),
            ),
            OutlinedButton.icon(
              onPressed: isMutating ? null : onCreateProduct,
              icon: const Icon(Icons.storefront_outlined),
              label: const Text('上架商品'),
            ),
          ],
        ),
      ],
    );
  }
}

class _WalletSummary extends StatelessWidget {
  const _WalletSummary({required this.snapshot});

  final WalletSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final LedgerVerify verification = snapshot.verification;
    final List<Widget> cards = <Widget>[
      _SummaryCard(
        icon: Icons.account_balance_wallet_outlined,
        label: '当前余额',
        value: '${snapshot.wallet.balance}',
        detail: '平台闭环积分',
      ),
      _SummaryCard(
        icon: verification.ok == true
            ? Icons.verified_user_outlined
            : Icons.gpp_maybe_outlined,
        label: '账本校验',
        value: verification.ok == true ? '通过' : '未通过或未知',
        detail:
            'seq ${verification.latestSeq ?? 0} · ${_shortHash(verification.latestHash)}',
      ),
      const _SummaryCard(
        icon: Icons.policy_outlined,
        label: '合规边界',
        value: '无充值/提现/自由转账',
        detail: '仅在内容与托管流程内流转',
      ),
    ];
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < 760) {
          return Column(
            children: cards
                .map(
                  (Widget card) => Padding(
                    padding: const EdgeInsets.only(bottom: 10),
                    child: card,
                  ),
                )
                .toList(growable: false),
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: cards
              .map(
                (Widget card) => Expanded(
                  child: Padding(
                    padding: const EdgeInsets.only(right: 10),
                    child: card,
                  ),
                ),
              )
              .toList(growable: false),
        );
      },
    );
  }
}

class _SummaryCard extends StatelessWidget {
  const _SummaryCard({
    required this.icon,
    required this.label,
    required this.value,
    required this.detail,
  });

  final IconData icon;
  final String label;
  final String value;
  final String detail;

  @override
  Widget build(BuildContext context) {
    final YourTjPalette palette = Theme.of(context).extension<YourTjPalette>()!;
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Row(
          children: <Widget>[
            DecoratedBox(
              decoration: BoxDecoration(
                color: palette.secondary,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Icon(icon, color: palette.primary),
              ),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(label, style: Theme.of(context).textTheme.labelLarge),
                  const SizedBox(height: 4),
                  Text(
                    value,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  Text(detail, style: Theme.of(context).textTheme.bodySmall),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _WalletKeyPanel extends StatelessWidget {
  const _WalletKeyPanel({
    required this.publicKey,
    required this.isMutating,
    required this.onBind,
    required this.onDelete,
    required this.onClaim,
  });

  final String? publicKey;
  final bool isMutating;
  final VoidCallback onBind;
  final VoidCallback onDelete;
  final VoidCallback onClaim;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text('本机签名钱包', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 6),
            const Text('私钥仅保存在本机 Keychain/Keystore；平台只接收公钥。'),
            const SizedBox(height: 12),
            SelectableText(
              publicKey ?? '本机还没有钱包密钥。首次绑定会在安全存储中生成 Ed25519 密钥。',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 14),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.tonalIcon(
                  onPressed: isMutating ? null : onBind,
                  icon: const Icon(Icons.key_rounded),
                  label: Text(publicKey == null ? '生成并绑定' : '确认当前公钥'),
                ),
                OutlinedButton.icon(
                  onPressed: isMutating ? null : onClaim,
                  icon: const Icon(Icons.merge_type_rounded),
                  label: const Text('认领旧钱包'),
                ),
                if (publicKey != null)
                  TextButton.icon(
                    onPressed: isMutating ? null : onDelete,
                    icon: const Icon(Icons.delete_outline_rounded),
                    label: const Text('清除本机私钥'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

Future<bool> _confirmWalletDeletion(BuildContext context) async {
  return await showDialog<bool>(
        context: context,
        builder: (BuildContext context) => AlertDialog(
          title: const Text('清除本机钱包私钥？'),
          content: const Text(
            '此操作不可撤销。服务端绑定的公钥不会随本机数据清除；清除后，本机无法再签署积分操作，当前也不能只凭登录态恢复或轮换钱包密钥。',
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('确认清除'),
            ),
          ],
        ),
      ) ??
      false;
}

String _shortHash(String? value) {
  if (value == null || value.isEmpty) {
    return '—';
  }
  if (value.length <= 12) {
    return value;
  }
  return '${value.substring(0, 6)}…${value.substring(value.length - 6)}';
}
