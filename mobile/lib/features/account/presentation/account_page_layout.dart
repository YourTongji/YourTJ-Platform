import 'package:flutter/material.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';

class AccountPageLayout extends StatelessWidget {
  const AccountPageLayout({
    required this.title,
    required this.child,
    this.actions,
    this.maxWidth = 720,
    super.key,
  });

  final String title;
  final Widget child;
  final List<Widget>? actions;
  final double maxWidth;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(title), actions: actions),
      body: SafeArea(
        top: false,
        child: Center(
          child: ConstrainedBox(
            constraints: BoxConstraints(maxWidth: maxWidth),
            child: child,
          ),
        ),
      ),
    );
  }
}

class AccountFailureView extends StatelessWidget {
  const AccountFailureView({required this.failure, this.onRetry, super.key});

  final ApiFailure failure;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    if (failure.kind == ApiFailureKind.unauthorized) {
      return const AppPermissionState(
        title: '需要登录',
        description: '该功能只对已登录账号开放，请从账号页登录后重试。',
      );
    }
    if (failure.kind == ApiFailureKind.forbidden) {
      return AppPermissionState(title: '当前不可访问', description: failure.message);
    }
    return AppErrorState(description: failure.message, onRetry: onRetry);
  }
}

String formatAccountTime(int seconds) {
  final DateTime time = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  String twoDigits(int value) => value.toString().padLeft(2, '0');
  return '${time.year}-${twoDigits(time.month)}-${twoDigits(time.day)} '
      '${twoDigits(time.hour)}:${twoDigits(time.minute)}';
}
