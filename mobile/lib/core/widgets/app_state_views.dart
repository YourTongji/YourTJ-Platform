import 'package:flutter/material.dart';

import '../design/app_theme.dart';
import '../l10n/app_strings.dart';

class AppLoadingState extends StatelessWidget {
  const AppLoadingState({
    this.title = AppStrings.loadingTitle,
    this.description = AppStrings.loadingDescription,
    super.key,
  });

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    return _StatePanel(
      icon: const SizedBox.square(
        dimension: 32,
        child: CircularProgressIndicator(strokeWidth: 3),
      ),
      title: title,
      description: description,
      isLiveRegion: true,
    );
  }
}

class AppEmptyState extends StatelessWidget {
  const AppEmptyState({
    this.title = AppStrings.emptyTitle,
    this.description = AppStrings.emptyDescription,
    this.action,
    super.key,
  });

  final String title;
  final String description;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return _StatePanel(
      icon: const Icon(Icons.inbox_outlined, size: 40),
      title: title,
      description: description,
      action: action,
    );
  }
}

class AppErrorState extends StatelessWidget {
  const AppErrorState({
    this.title = AppStrings.errorTitle,
    this.description = AppStrings.errorDescription,
    this.onRetry,
    super.key,
  });

  final String title;
  final String description;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final YourTjPalette palette = Theme.of(context).extension<YourTjPalette>()!;
    return _StatePanel(
      icon: Icon(
        Icons.error_outline_rounded,
        size: 40,
        color: palette.destructive,
      ),
      title: title,
      description: description,
      isLiveRegion: true,
      action: onRetry == null
          ? null
          : FilledButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh_rounded),
              label: const Text(AppStrings.retry),
            ),
    );
  }
}

class AppPermissionState extends StatelessWidget {
  const AppPermissionState({
    this.title = AppStrings.permissionTitle,
    this.description = AppStrings.permissionDescription,
    super.key,
  });

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    return _StatePanel(
      icon: const Icon(Icons.lock_outline_rounded, size: 40),
      title: title,
      description: description,
      isLiveRegion: true,
    );
  }
}

class _StatePanel extends StatelessWidget {
  const _StatePanel({
    required this.icon,
    required this.title,
    required this.description,
    this.action,
    this.isLiveRegion = false,
  });

  final Widget icon;
  final String title;
  final String description;
  final Widget? action;
  final bool isLiveRegion;

  @override
  Widget build(BuildContext context) {
    final YourTjPalette palette = Theme.of(context).extension<YourTjPalette>()!;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Semantics(
                key: const Key('app-state-announcement'),
                container: true,
                liveRegion: isLiveRegion,
                label: '$title，$description',
                child: ExcludeSemantics(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      IconTheme(
                        data: IconThemeData(color: palette.onMuted),
                        child: icon,
                      ),
                      const SizedBox(height: 16),
                      Text(
                        title,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 8),
                      Text(
                        description,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: palette.onMuted,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              if (action != null) ...<Widget>[
                const SizedBox(height: 20),
                action!,
              ],
            ],
          ),
        ),
      ),
    );
  }
}
