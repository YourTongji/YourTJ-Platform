import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../auth/domain/session_state.dart';
import '../../settings/presentation/recent_auth_dialog.dart';
import '../data/admin_repository.dart';
import '../domain/admin_capabilities.dart';
import '../domain/admin_mutations.dart';
import 'admin_mutation_dialog.dart';
import 'admin_providers.dart';

class AdminCenterPage extends ConsumerWidget {
  const AdminCenterPage({this.requestedSectionPath, super.key});

  final String? requestedSectionPath;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('管理中心')),
      body: SafeArea(
        top: false,
        child: session.when(
          loading: () => const _LoadingAccess(),
          error: (Object error, StackTrace stackTrace) => const _AccessDenied(
            title: '无法确认管理权限',
            message: '登录状态读取失败。为避免越权，管理中心已拒绝访问。',
          ),
          data: (SessionState state) =>
              _buildAuthenticated(context, ref, state),
        ),
      ),
    );
  }

  Widget _buildAuthenticated(
    BuildContext context,
    WidgetRef ref,
    SessionState state,
  ) {
    final Account? account = state.account;
    if (!state.isAuthenticated || account == null) {
      return _AccessDenied(
        title: '需要登录',
        message: '管理能力由服务器随账号会话签发，匿名状态不能访问。',
        actionLabel: '前往登录',
        onAction: () => context.go(AppRoutes.login),
      );
    }
    final List<AdminModule> modules = adminModulesForCapabilities(
      account.capabilities,
    );
    if (modules.isEmpty) {
      return const _AccessDenied(
        title: '没有管理权限',
        message: '当前账号没有任何已知管理能力。客户端不会根据角色推断权限。',
      );
    }
    final AdminSection? section = AdminSection.fromPathSegment(
      requestedSectionPath,
    );
    if (requestedSectionPath != null && section == null) {
      return const _AccessDenied(
        title: '管理模块不存在',
        message: '深链接中的管理模块无法识别，客户端已拒绝继续访问。',
      );
    }
    if (section != null &&
        !modules.any((AdminModule module) => module.section == section)) {
      return const _AccessDenied(
        title: '无权访问此模块',
        message: '深链接指向的模块不在服务器签发的 capabilities 中，访问已被拒绝。',
      );
    }
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth >= 840) {
          final AdminSection selected = section ?? modules.first.section;
          return _TabletAdminLayout(modules: modules, selected: selected);
        }
        if (section == null) {
          return _MobileModuleList(account: account, modules: modules);
        }
        return AdminSectionPanel(section: section);
      },
    );
  }
}

class _TabletAdminLayout extends StatelessWidget {
  const _TabletAdminLayout({required this.modules, required this.selected});

  final List<AdminModule> modules;
  final AdminSection selected;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        SizedBox(
          width: 280,
          child: Material(
            color: Theme.of(context).colorScheme.surfaceContainerLow,
            child: ListView(
              padding: const EdgeInsets.all(12),
              children: <Widget>[
                const Padding(
                  padding: EdgeInsets.fromLTRB(12, 8, 12, 16),
                  child: Text('按服务器能力显示'),
                ),
                for (final AdminModule module in modules)
                  ListTile(
                    selected: module.section == selected,
                    leading: Icon(_iconFor(module.section)),
                    title: Text(module.label),
                    subtitle: Text(module.description),
                    onTap: () => context.go(
                      AppRoutes.adminSection(module.section.pathSegment),
                    ),
                  ),
              ],
            ),
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(child: AdminSectionPanel(section: selected)),
      ],
    );
  }
}

class _MobileModuleList extends StatelessWidget {
  const _MobileModuleList({required this.account, required this.modules});

  final Account account;
  final List<AdminModule> modules;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        Text('管理后台', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 4),
        Text('@${account.handle} · ${account.capabilities.length} 项服务端能力'),
        const SizedBox(height: 16),
        const _AdminSafetyNotice(),
        const SizedBox(height: 12),
        for (final AdminModule module in modules)
          Card(
            child: ListTile(
              leading: Icon(_iconFor(module.section)),
              title: Text(module.label),
              subtitle: Text(module.description),
              trailing: const Icon(Icons.chevron_right_rounded),
              onTap: () => context.push(
                AppRoutes.adminSection(module.section.pathSegment),
              ),
            ),
          ),
      ],
    );
  }
}

class AdminSectionPanel extends ConsumerWidget {
  const AdminSectionPanel({required this.section, super.key});

  final AdminSection section;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final AdminModule module = adminModuleForSection(section)!;
    final AsyncValue<AdminSectionSnapshot> snapshot = ref.watch(
      adminSectionSnapshotProvider(section),
    );
    return RefreshIndicator(
      onRefresh: () =>
          ref.refresh(adminSectionSnapshotProvider(section).future),
      child: CustomScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        slivers: <Widget>[
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 20, 16, 8),
            sliver: SliverToBoxAdapter(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Row(
                    children: <Widget>[
                      Icon(_iconFor(section)),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          module.label,
                          style: Theme.of(context).textTheme.headlineSmall,
                        ),
                      ),
                      IconButton(
                        tooltip: '刷新',
                        onPressed: () => ref.invalidate(
                          adminSectionSnapshotProvider(section),
                        ),
                        icon: const Icon(Icons.refresh_rounded),
                      ),
                    ],
                  ),
                  Text(module.description),
                  const SizedBox(height: 12),
                  const _AdminSafetyNotice(),
                  const SizedBox(height: 8),
                  Text(
                    module.mobileCoverage,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ),
          ...snapshot.when(
            loading: () => const <Widget>[
              SliverFillRemaining(
                hasScrollBody: false,
                child: Center(child: CircularProgressIndicator()),
              ),
            ],
            error: (Object error, StackTrace stackTrace) => <Widget>[
              SliverFillRemaining(
                hasScrollBody: false,
                child: _AdminLoadError(
                  error: error,
                  onRetry: () =>
                      ref.invalidate(adminSectionSnapshotProvider(section)),
                  onRecentAuth: error is AdminRecentAuthenticationRequired
                      ? () => _authenticateAndRetry(context, ref)
                      : null,
                ),
              ),
            ],
            data: (AdminSectionSnapshot data) =>
                _snapshotSlivers(context, ref, data),
          ),
        ],
      ),
    );
  }

  Future<void> _authenticateAndRetry(
    BuildContext context,
    WidgetRef ref,
  ) async {
    final bool authenticated = await ensureRecentAuthentication(context, ref);
    if (authenticated) {
      ref.invalidate(adminSectionSnapshotProvider(section));
    }
  }

  List<Widget> _snapshotSlivers(
    BuildContext context,
    WidgetRef ref,
    AdminSectionSnapshot snapshot,
  ) {
    if (snapshot.groups.isEmpty && snapshot.actions.isEmpty) {
      return const <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: Center(child: Text('当前能力范围没有可读取的数据源')),
        ),
      ];
    }
    return <Widget>[
      if (snapshot.actions.isNotEmpty)
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          sliver: SliverToBoxAdapter(
            child: _AdminActionsCard(
              actions: snapshot.actions,
              onAction: (AdminMutationAction action) =>
                  _runMutation(context, ref, action),
            ),
          ),
        ),
      for (final AdminRecordGroup group in snapshot.groups)
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          sliver: SliverToBoxAdapter(
            child: _AdminGroupCard(
              group: group,
              onAction: (AdminMutationAction action) =>
                  _runMutation(context, ref, action),
            ),
          ),
        ),
      SliverPadding(
        padding: const EdgeInsets.all(20),
        sliver: SliverToBoxAdapter(
          child: Text(
            '读取于 ${TimeOfDay.fromDateTime(snapshot.loadedAt).format(context)}',
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ),
      ),
    ];
  }

  Future<void> _runMutation(
    BuildContext context,
    WidgetRef ref,
    AdminMutationAction action,
  ) async {
    final AdminMutationSubmission? submission = await showAdminMutationDialog(
      context: context,
      action: action,
    );
    if (submission == null || !context.mounted) {
      return;
    }
    if (action.requiresRecentAuth &&
        !await ensureRecentAuthentication(context, ref)) {
      return;
    }
    final SessionState session = await ref.read(sessionStateProvider.future);
    final Account? account = session.account;
    if (!context.mounted || !session.isAuthenticated || account == null) {
      if (context.mounted) {
        _showMessage(context, '登录状态已失效，管理操作没有发送');
      }
      return;
    }
    unawaited(
      showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (BuildContext context) => const _MutationProgressDialog(),
      ),
    );
    try {
      final AdminMutationResult result = await ref
          .read(adminMutationExecutorProvider)
          .execute(
            action,
            submission,
            AdminActorContext(
              accountId: account.id,
              role: account.role.value,
              capabilities: account.capabilities.toSet(),
            ),
          );
      if (!context.mounted) {
        return;
      }
      Navigator.of(context, rootNavigator: true).pop();
      _refreshAfterMutation(ref);
      final bytes = result.previewBytes;
      if (bytes != null) {
        await showDialog<void>(
          context: context,
          builder: (BuildContext context) => AlertDialog(
            title: const Text('一次性安全预览'),
            content: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 720, maxHeight: 720),
              child: Image.memory(
                bytes,
                fit: BoxFit.contain,
                errorBuilder:
                    (BuildContext context, Object error, StackTrace? stack) =>
                        const Text('预览内容已安全加载，但当前格式无法在移动端直接显示。'),
              ),
            ),
            actions: <Widget>[
              FilledButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('关闭'),
              ),
            ],
          ),
        );
      } else {
        _showMessage(context, result.message);
      }
    } on AdminRecentAuthenticationRequired {
      if (context.mounted) {
        Navigator.of(context, rootNavigator: true).pop();
        _showMessage(context, '近期认证已过期。请求没有重试；请重新审阅证据并发起操作。');
      }
    } on ApiFailure catch (failure) {
      if (context.mounted) {
        Navigator.of(context, rootNavigator: true).pop();
        if (failure.kind == ApiFailureKind.conflict) {
          ref.invalidate(adminSectionSnapshotProvider(section));
          _showMessage(context, '数据已被其他管理员修改。已刷新证据，请重新填写并确认。');
        } else {
          _showMessage(context, failure.message);
        }
      }
    } on AdminMutationValidation catch (failure) {
      if (context.mounted) {
        Navigator.of(context, rootNavigator: true).pop();
        _showMessage(context, failure.message);
      }
    } on AdminAccessDenied {
      if (context.mounted) {
        Navigator.of(context, rootNavigator: true).pop();
        _showMessage(context, '服务器能力或管理层级不允许此操作，请刷新后重试');
      }
    } on Object {
      if (context.mounted) {
        Navigator.of(context, rootNavigator: true).pop();
        _showMessage(context, '管理操作失败，服务器没有返回可安全展示的详情');
      }
    }
  }

  void _showMessage(BuildContext context, String message) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(message)));
  }

  void _refreshAfterMutation(WidgetRef ref) {
    ref.invalidate(adminSectionSnapshotProvider(section));
    if (section != AdminSection.overview) {
      ref.invalidate(adminSectionSnapshotProvider(AdminSection.overview));
    }
  }
}

class _AdminGroupCard extends StatelessWidget {
  const _AdminGroupCard({required this.group, required this.onAction});

  final AdminRecordGroup group;
  final ValueChanged<AdminMutationAction> onAction;

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    group.title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                if (group.hasMore) const Chip(label: Text('还有更多')),
              ],
            ),
            if (group.description != null) ...<Widget>[
              const SizedBox(height: 4),
              Text(group.description!),
            ],
            if (group.records.isEmpty) ...<Widget>[
              const SizedBox(height: 16),
              const Text('当前队列为空'),
            ],
            for (final AdminRecord record in group.records) ...<Widget>[
              const Divider(height: 24),
              Semantics(
                container: true,
                label: '${record.title}，${record.subtitle}',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Text(
                      record.title,
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    const SizedBox(height: 4),
                    Text(record.subtitle),
                    if (record.evidence.isNotEmpty) ...<Widget>[
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: record.evidence
                            .map((String item) => Chip(label: Text(item)))
                            .toList(growable: false),
                      ),
                    ],
                    if (record.actions.isNotEmpty) ...<Widget>[
                      const SizedBox(height: 10),
                      _AdminActionButtons(
                        actions: record.actions,
                        onAction: onAction,
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _AdminActionsCard extends StatelessWidget {
  const _AdminActionsCard({required this.actions, required this.onAction});

  final List<AdminMutationAction> actions;
  final ValueChanged<AdminMutationAction> onAction;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('可执行操作', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            const Text('只显示服务器 capabilities 允许的操作；提交时服务器仍会再次鉴权。'),
            const SizedBox(height: 12),
            _AdminActionButtons(actions: actions, onAction: onAction),
          ],
        ),
      ),
    );
  }
}

class _AdminActionButtons extends StatelessWidget {
  const _AdminActionButtons({required this.actions, required this.onAction});

  final List<AdminMutationAction> actions;
  final ValueChanged<AdminMutationAction> onAction;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: <Widget>[
        for (final AdminMutationAction action in actions)
          action.isDestructive
              ? OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Theme.of(context).colorScheme.error,
                  ),
                  onPressed: () => onAction(action),
                  icon: const Icon(Icons.warning_amber_rounded),
                  label: Text(action.label),
                )
              : FilledButton.tonalIcon(
                  onPressed: () => onAction(action),
                  icon: const Icon(Icons.edit_outlined),
                  label: Text(action.label),
                ),
      ],
    );
  }
}

class _MutationProgressDialog extends StatelessWidget {
  const _MutationProgressDialog();

  @override
  Widget build(BuildContext context) {
    return const AlertDialog(
      content: Row(
        children: <Widget>[
          CircularProgressIndicator(),
          SizedBox(width: 20),
          Expanded(child: Text('正在提交；此请求不会自动重试…')),
        ],
      ),
    );
  }
}

class _AdminSafetyNotice extends StatelessWidget {
  const _AdminSafetyNotice();

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Theme.of(context).colorScheme.secondaryContainer,
      child: const Padding(
        padding: EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(Icons.verified_user_outlined),
            SizedBox(width: 10),
            Expanded(child: Text('管理操作会要求完整理由、近期认证与明确确认；版本冲突或认证过期时不会自动重试。')),
          ],
        ),
      ),
    );
  }
}

class _AdminLoadError extends StatelessWidget {
  const _AdminLoadError({
    required this.error,
    required this.onRetry,
    this.onRecentAuth,
  });

  final Object error;
  final VoidCallback onRetry;
  final VoidCallback? onRecentAuth;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            const Icon(Icons.error_outline_rounded, size: 48),
            const SizedBox(height: 12),
            Text(
              error is AdminRecentAuthenticationRequired
                  ? '需要近期身份验证'
                  : '无法读取管理数据',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 8),
            Text(_safeMessage(error), textAlign: TextAlign.center),
            const SizedBox(height: 16),
            if (onRecentAuth != null)
              FilledButton(onPressed: onRecentAuth, child: const Text('完成近期认证'))
            else
              FilledButton(onPressed: onRetry, child: const Text('重试')),
          ],
        ),
      ),
    );
  }

  static String _safeMessage(Object error) => switch (error) {
    AdminRecentAuthenticationRequired() => '服务器要求重新确认当前管理会话。完成认证后再刷新。',
    AdminAccessDenied() => '当前账号不再具备这个模块所需的服务器能力。',
    ApiFailure failure => failure.message,
    _ => '管理数据响应无法安全解析；请刷新，持续失败时联系平台维护人员。',
  };
}

class _AccessDenied extends StatelessWidget {
  const _AccessDenied({
    required this.title,
    required this.message,
    this.actionLabel,
    this.onAction,
  });

  final String title;
  final String message;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 480),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              const Icon(Icons.lock_outline_rounded, size: 56),
              const SizedBox(height: 16),
              Text(title, style: Theme.of(context).textTheme.headlineSmall),
              const SizedBox(height: 8),
              Text(message, textAlign: TextAlign.center),
              if (actionLabel != null && onAction != null) ...<Widget>[
                const SizedBox(height: 20),
                FilledButton(onPressed: onAction, child: Text(actionLabel!)),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _LoadingAccess extends StatelessWidget {
  const _LoadingAccess();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          CircularProgressIndicator(),
          SizedBox(height: 12),
          Text('确认管理权限'),
        ],
      ),
    );
  }
}

IconData _iconFor(AdminSection section) => switch (section) {
  AdminSection.overview => Icons.dashboard_outlined,
  AdminSection.users => Icons.people_outline_rounded,
  AdminSection.moderation => Icons.shield_outlined,
  AdminSection.appeals => Icons.gavel_outlined,
  AdminSection.resources => Icons.category_outlined,
  AdminSection.activity => Icons.insights_outlined,
  AdminSection.announcements => Icons.campaign_outlined,
  AdminSection.promotions => Icons.view_carousel_outlined,
  AdminSection.achievements => Icons.emoji_events_outlined,
  AdminSection.verifications => Icons.verified_outlined,
  AdminSection.creditIntegrity => Icons.balance_outlined,
  AdminSection.audit => Icons.history_outlined,
  AdminSection.system => Icons.settings_suggest_outlined,
};
