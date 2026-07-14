import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/widgets/platform_avatar.dart';
import '../../admin/domain/admin_capabilities.dart';
import '../../auth/domain/session_state.dart';

class AccountPage extends ConsumerWidget {
  const AccountPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('账号与设置')),
      body: SafeArea(
        top: false,
        child: session.when(
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (Object error, StackTrace stackTrace) => _SessionNotice(
            icon: Icons.sync_problem_rounded,
            title: '无法读取登录状态',
            message: '请重新打开页面后重试。',
            actionLabel: '重试',
            onAction: () => ref.invalidate(sessionStateProvider),
          ),
          data: (SessionState state) => switch (state.phase) {
            SessionPhase.restoring => const Center(
              child: CircularProgressIndicator(),
            ),
            SessionPhase.anonymous => _AnonymousAccount(
              onLogin: () => context.push(AppRoutes.login),
              onRecover: () => context.push(AppRoutes.recovery),
            ),
            SessionPhase.authenticated => _AuthenticatedAccount(
              account: state.account!,
              onRefreshAvatar: () => ref.invalidate(sessionStateProvider),
              onLogout: () => _logout(context, ref),
              onOnboarding: () => context.push(AppRoutes.onboarding),
              onProfile: () =>
                  context.push(AppRoutes.profile(state.account!.handle)),
              onSettings: () => context.push(AppRoutes.settings),
              onAppeals: () => context.push(AppRoutes.appeals),
              onAdmin: () => context.push(AppRoutes.admin),
            ),
            SessionPhase.reconnectRequired => _SessionNotice(
              icon: Icons.cloud_off_rounded,
              title: '暂时无法恢复登录',
              message: state.message ?? '请检查网络后重试。公开内容仍可继续浏览。',
              actionLabel: '重新连接',
              onAction: ref.read(sessionManagerProvider).retrySession,
            ),
            SessionPhase.secureStorageUnavailable => _SessionNotice(
              icon: Icons.phonelink_lock_rounded,
              title: '安全存储不可用',
              message: state.message ?? '为保护账号，本机不会降级保存登录凭证。',
            ),
          },
        ),
      ),
    );
  }

  Future<void> _logout(BuildContext context, WidgetRef ref) async {
    final bool revoked = await ref.read(sessionManagerProvider).logout();
    if (!context.mounted) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(revoked ? '已安全退出当前设备' : '已清除本机登录；服务器撤销暂时无法确认')),
    );
  }
}

class _AnonymousAccount extends StatelessWidget {
  const _AnonymousAccount({required this.onLogin, required this.onRecover});

  final VoidCallback onLogin;
  final VoidCallback onRecover;

  @override
  Widget build(BuildContext context) {
    return _AccountBody(
      children: <Widget>[
        const Icon(Icons.account_circle_outlined, size: 64),
        const SizedBox(height: 16),
        Text('尚未登录', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        const Text('公开论坛、课程和评课可直接浏览；互动、消息、课表与积分需要登录。'),
        const SizedBox(height: 24),
        FilledButton.icon(
          onPressed: onLogin,
          icon: const Icon(Icons.login_rounded),
          label: const Text('登录或注册'),
        ),
        const SizedBox(height: 12),
        TextButton.icon(
          onPressed: onRecover,
          icon: const Icon(Icons.restore_rounded),
          label: const Text('恢复已停用或待删除账号'),
        ),
      ],
    );
  }
}

class _AuthenticatedAccount extends StatelessWidget {
  const _AuthenticatedAccount({
    required this.account,
    required this.onRefreshAvatar,
    required this.onLogout,
    required this.onOnboarding,
    required this.onProfile,
    required this.onSettings,
    required this.onAppeals,
    required this.onAdmin,
  });

  final Account account;
  final VoidCallback onRefreshAvatar;
  final VoidCallback onLogout;
  final VoidCallback onOnboarding;
  final VoidCallback onProfile;
  final VoidCallback onSettings;
  final VoidCallback onAppeals;
  final VoidCallback onAdmin;

  @override
  Widget build(BuildContext context) {
    final bool hasStaffCapabilities = adminModulesForCapabilities(
      account.capabilities,
    ).isNotEmpty;
    return _AccountBody(
      children: <Widget>[
        PlatformAvatar(
          radius: 36,
          // Account exposes only this compatibility field until its typed delivery lands.
          // ignore: deprecated_member_use
          compatibilityUrl: account.avatarUrl,
          fallbackText: account.handle,
          semanticLabel: '${account.handle} 的头像',
          onRefresh: onRefreshAvatar,
        ),
        const SizedBox(height: 16),
        Semantics(
          header: true,
          child: Text(
            '@${account.handle}',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
        ),
        const SizedBox(height: 6),
        Text('信任等级 ${account.trustLevel}'),
        if (account.onboardingRequired) ...<Widget>[
          const SizedBox(height: 16),
          Card(
            child: ListTile(
              leading: const Icon(Icons.assignment_turned_in_outlined),
              title: const Text('需要完成首次设置'),
              subtitle: const Text('请先确认资料、隐私选择与当前条款。'),
              trailing: const Icon(Icons.chevron_right_rounded),
              onTap: onOnboarding,
            ),
          ),
        ],
        const SizedBox(height: 24),
        Card(
          child: Column(
            children: <Widget>[
              ListTile(
                leading: const Icon(Icons.person_outline_rounded),
                title: const Text('个人资料'),
                subtitle: const Text('资料、社交关系与公开内容'),
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: onProfile,
              ),
              const Divider(height: 1),
              ListTile(
                leading: const Icon(Icons.settings_outlined),
                title: const Text('设置与账号安全'),
                subtitle: const Text('隐私、通知、会话、导出与账号生命周期'),
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: onSettings,
              ),
              const Divider(height: 1),
              ListTile(
                leading: const Icon(Icons.gavel_outlined),
                title: const Text('申诉中心'),
                subtitle: const Text('查看本人治理事件、申诉进度与可用操作'),
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: onAppeals,
              ),
              if (hasStaffCapabilities) ...<Widget>[
                const Divider(height: 1),
                ListTile(
                  leading: const Icon(Icons.admin_panel_settings_outlined),
                  title: const Text('管理中心'),
                  subtitle: Text('已授权 ${account.capabilities.length} 项能力'),
                  trailing: const Icon(Icons.chevron_right_rounded),
                  onTap: onAdmin,
                ),
              ],
            ],
          ),
        ),
        const SizedBox(height: 24),
        OutlinedButton.icon(
          onPressed: onLogout,
          icon: const Icon(Icons.logout_rounded),
          label: const Text('退出当前设备'),
        ),
      ],
    );
  }
}

class _SessionNotice extends StatelessWidget {
  const _SessionNotice({
    required this.icon,
    required this.title,
    required this.message,
    this.actionLabel,
    this.onAction,
  });

  final IconData icon;
  final String title;
  final String message;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    return _AccountBody(
      children: <Widget>[
        Icon(icon, size: 56),
        const SizedBox(height: 16),
        Text(title, style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        Text(message),
        if (actionLabel != null && onAction != null) ...<Widget>[
          const SizedBox(height: 24),
          FilledButton(onPressed: onAction, child: Text(actionLabel!)),
        ],
      ],
    );
  }
}

class _AccountBody extends StatelessWidget {
  const _AccountBody({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: children,
          ),
        ),
      ),
    );
  }
}
