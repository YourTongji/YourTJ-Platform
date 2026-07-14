enum WindowSizeClass { compact, medium, expanded }

abstract final class AdaptiveBreakpoints {
  static const double navigationRail = 600;
  static const double expandedLayout = 840;

  static WindowSizeClass classify(double width) {
    if (width < navigationRail) {
      return WindowSizeClass.compact;
    }
    if (width < expandedLayout) {
      return WindowSizeClass.medium;
    }
    return WindowSizeClass.expanded;
  }
}
