import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/design/theme_mode_controller.dart';
import '../../auth/domain/session_state.dart';

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final SessionState session =
        ref.watch(sessionStateProvider).value ??
        ref.read(sessionManagerProvider).state;
    final bool isAuthenticated = session.isAuthenticated;
    return Scaffold(
      appBar: AppBar(title: const Text('设置与账号安全')),
      body: SafeArea(
        top: false,
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 760),
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                _ThemeModeCard(controller: ThemeModeScope.of(context)),
                if (!isAuthenticated)
                  Card(
                    child: ListTile(
                      leading: const Icon(Icons.login_rounded),
                      title: const Text('登录后管理账号'),
                      subtitle: const Text('恢复已停用/待删除账号不需要普通登录。'),
                      onTap: () => context.push(AppRoutes.login),
                    ),
                  ),
                if (isAuthenticated) ...<Widget>[
                  if (session.account?.onboardingRequired == true)
                    _SettingsTile(
                      icon: Icons.assignment_turned_in_outlined,
                      title: '完成首次设置',
                      description: '确认公开 handle、资料、隐私与当前条款。',
                      onTap: () => context.push(AppRoutes.onboarding),
                    ),
                  _SettingsTile(
                    icon: Icons.person_outline_rounded,
                    title: '个人资料',
                    description: 'Handle、显示名、学校、简介与 HTTPS 网站。',
                    onTap: () => context.push(AppRoutes.profileSettings),
                  ),
                  _SettingsTile(
                    icon: Icons.shield_outlined,
                    title: '隐私与社交权限',
                    description: '资料、活动、关系列表、私信、提及与发现。',
                    onTap: () => context.push(AppRoutes.privacySettings),
                  ),
                  _SettingsTile(
                    icon: Icons.notifications_outlined,
                    title: '通知偏好',
                    description: '应用内各类通知与每周邮件摘要。',
                    onTap: () => context.push(AppRoutes.notificationSettings),
                  ),
                  _SettingsTile(
                    icon: Icons.devices_rounded,
                    title: '设备与会话',
                    description: '查看当前/其他设备，撤销单个或其他全部会话。',
                    onTap: () => context.push(AppRoutes.sessions),
                  ),
                  _SettingsTile(
                    icon: Icons.password_rounded,
                    title: session.account?.hasPassword == true
                        ? '修改密码'
                        : '设置密码',
                    description: '更新凭据时安全替换本机 session，并撤销旧 refresh family。',
                    onTap: () => context.push(AppRoutes.passwordSettings),
                  ),
                  _SettingsTile(
                    icon: Icons.archive_outlined,
                    title: '我的数据导出',
                    description: '最近认证后创建、查看与下载有限期 owner export。',
                    onTap: () => context.push(AppRoutes.dataExports),
                  ),
                  _SettingsTile(
                    icon: Icons.manage_accounts_outlined,
                    title: '停用或删除账号',
                    description: '查看明确影响、二次确认、最近认证与恢复窗口。',
                    onTap: () => context.push(AppRoutes.lifecycle),
                  ),
                ],
                _SettingsTile(
                  icon: Icons.restore_rounded,
                  title: '恢复已停用或待删除账号',
                  description: '使用密码或 recovery-purpose 邮箱验证，不创建普通会话。',
                  onTap: () => context.push(AppRoutes.recovery),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ThemeModeCard extends StatelessWidget {
  const _ThemeModeCard({required this.controller});

  final ThemeModeController controller;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('外观', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            const Text('跟随系统，或为 YourTJ 单独选择浅色/深色。'),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                _ThemeChoice(
                  mode: ThemeMode.system,
                  selectedMode: controller.mode,
                  icon: Icons.brightness_auto_outlined,
                  label: '系统',
                  onSelected: controller.setMode,
                ),
                _ThemeChoice(
                  mode: ThemeMode.light,
                  selectedMode: controller.mode,
                  icon: Icons.light_mode_outlined,
                  label: '浅色',
                  onSelected: controller.setMode,
                ),
                _ThemeChoice(
                  mode: ThemeMode.dark,
                  selectedMode: controller.mode,
                  icon: Icons.dark_mode_outlined,
                  label: '深色',
                  onSelected: controller.setMode,
                ),
              ],
            ),
            if (controller.persistenceFailure
                case final ThemeModePersistenceFailure failure) ...<Widget>[
              const SizedBox(height: 12),
              Semantics(
                container: true,
                liveRegion: true,
                child: DecoratedBox(
                  key: const Key('theme-persistence-error'),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.errorContainer,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.sync_problem_rounded,
                          color: Theme.of(context).colorScheme.onErrorContainer,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(switch (failure) {
                            ThemeModePersistenceFailure.load =>
                              '无法读取已保存的外观偏好，当前暂时跟随系统。',
                            ThemeModePersistenceFailure.save =>
                              '外观已在本次运行中切换，但未能保存；下次启动可能恢复原设置。',
                          }),
                        ),
                        const SizedBox(width: 8),
                        TextButton(
                          key: const Key('theme-persistence-retry'),
                          onPressed: controller.isPersisting
                              ? null
                              : () => unawaited(controller.retryPersistence()),
                          child: Text(
                            controller.isPersisting
                                ? '重试中'
                                : failure == ThemeModePersistenceFailure.load
                                ? '重新读取'
                                : '重试保存',
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _ThemeChoice extends StatelessWidget {
  const _ThemeChoice({
    required this.mode,
    required this.selectedMode,
    required this.icon,
    required this.label,
    required this.onSelected,
  });

  final ThemeMode mode;
  final ThemeMode selectedMode;
  final IconData icon;
  final String label;
  final Future<void> Function(ThemeMode) onSelected;

  @override
  Widget build(BuildContext context) {
    return ChoiceChip(
      selected: mode == selectedMode,
      avatar: Icon(icon, size: 18),
      label: Text(label),
      onSelected: (bool selected) {
        if (selected) {
          unawaited(onSelected(mode));
        }
      },
    );
  }
}

class _SettingsTile extends StatelessWidget {
  const _SettingsTile({
    required this.icon,
    required this.title,
    required this.description,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String description;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: Icon(icon),
        title: Text(title),
        subtitle: Text(description),
        trailing: const Icon(Icons.chevron_right_rounded),
        onTap: onTap,
      ),
    );
  }
}
