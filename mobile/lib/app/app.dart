import 'dart:async';

import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../core/design/app_theme.dart';
import '../core/design/theme_mode_controller.dart';
import '../core/l10n/app_strings.dart';
import '../features/announcements/presentation/global_announcement_gate.dart';
import 'router.dart';

class YourTjApp extends StatefulWidget {
  const YourTjApp({
    this.router,
    this.themeMode,
    this.enableAnnouncementGate = true,
    super.key,
  });

  final GoRouter? router;
  final ThemeMode? themeMode;
  final bool enableAnnouncementGate;

  @override
  State<YourTjApp> createState() => _YourTjAppState();
}

class _YourTjAppState extends State<YourTjApp> {
  late final ThemeModeController _themeModeController;

  @override
  void initState() {
    super.initState();
    _themeModeController = ThemeModeController(
      initialMode: widget.themeMode ?? ThemeMode.system,
    );
    if (widget.themeMode == null) {
      unawaited(_themeModeController.restore());
    }
  }

  @override
  void dispose() {
    _themeModeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final GoRouter resolvedRouter = widget.router ?? appRouter;
    return ThemeModeScope(
      controller: _themeModeController,
      child: ListenableBuilder(
        listenable: _themeModeController,
        builder: (BuildContext context, Widget? child) {
          return MaterialApp.router(
            title: AppStrings.appName,
            debugShowCheckedModeBanner: false,
            theme: AppTheme.light,
            darkTheme: AppTheme.dark,
            themeMode: widget.themeMode ?? _themeModeController.mode,
            routerConfig: resolvedRouter,
            builder: widget.enableAnnouncementGate
                ? (BuildContext context, Widget? child) {
                    return GlobalAnnouncementGate(
                      navigatorKey: resolvedRouter.routerDelegate.navigatorKey,
                      child: child ?? const SizedBox.shrink(),
                    );
                  }
                : null,
          );
        },
      ),
    );
  }
}
