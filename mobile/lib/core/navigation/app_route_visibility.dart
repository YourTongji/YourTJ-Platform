import 'package:flutter/widgets.dart';

/// Exposes whether the root application route is currently visible.
class AppRouteVisibilityScope extends InheritedWidget {
  const AppRouteVisibilityScope({
    required this.isVisible,
    required super.child,
    super.key,
  });

  final bool isVisible;

  static bool isVisibleOf(BuildContext context) {
    return context
            .dependOnInheritedWidgetOfExactType<AppRouteVisibilityScope>()
            ?.isVisible ??
        false;
  }

  @override
  bool updateShouldNotify(AppRouteVisibilityScope oldWidget) {
    return isVisible != oldWidget.isVisible;
  }
}
