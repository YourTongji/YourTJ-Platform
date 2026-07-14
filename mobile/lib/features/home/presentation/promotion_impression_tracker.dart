import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../core/navigation/app_route_visibility.dart';

/// Records an impression only after the child remains materially visible.
class PromotionImpressionTracker extends StatefulWidget {
  const PromotionImpressionTracker({
    required this.trackingToken,
    required this.onImpression,
    required this.child,
    this.minimumVisibleFraction = 0.5,
    this.dwellDuration = const Duration(milliseconds: 500),
    super.key,
  });

  final String? trackingToken;
  final VoidCallback onImpression;
  final Widget child;
  final double minimumVisibleFraction;
  final Duration dwellDuration;

  @override
  State<PromotionImpressionTracker> createState() =>
      _PromotionImpressionTrackerState();
}

class _PromotionImpressionTrackerState extends State<PromotionImpressionTracker>
    with WidgetsBindingObserver {
  ScrollableState? _scrollable;
  ValueListenable<TickerModeData>? _tickerMode;
  Timer? _dwellTimer;
  String? _recordedToken;
  bool _hasScheduledCheck = false;
  bool _isForeground = false;
  bool _isScopeVisible = false;
  bool _isRouteVisible = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _isForeground =
        WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed;
    _scheduleVisibilityCheck();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _isForeground = state == AppLifecycleState.resumed;
    if (_isForeground) {
      _scheduleVisibilityCheck();
    } else {
      _cancelDwell();
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _isScopeVisible = AppRouteVisibilityScope.isVisibleOf(context);
    final ValueListenable<TickerModeData> nextTickerMode =
        TickerMode.getValuesNotifier(context);
    if (nextTickerMode != _tickerMode) {
      _tickerMode?.removeListener(_handleTickerModeChanged);
      _tickerMode = nextTickerMode;
      _tickerMode?.addListener(_handleTickerModeChanged);
    }
    _updateRouteVisibility();
    final ScrollableState? nextScrollable = Scrollable.maybeOf(context);
    if (nextScrollable != _scrollable) {
      _scrollable?.position.removeListener(_scheduleVisibilityCheck);
      _scrollable = nextScrollable;
      _scrollable?.position.addListener(_scheduleVisibilityCheck);
    }
    _scheduleVisibilityCheck();
  }

  @override
  void didUpdateWidget(covariant PromotionImpressionTracker oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.trackingToken != widget.trackingToken) {
      _cancelDwell();
      _recordedToken = null;
    }
    _scheduleVisibilityCheck();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _tickerMode?.removeListener(_handleTickerModeChanged);
    _scrollable?.position.removeListener(_scheduleVisibilityCheck);
    _dwellTimer?.cancel();
    super.dispose();
  }

  void _scheduleVisibilityCheck() {
    if (_hasScheduledCheck) {
      return;
    }
    _hasScheduledCheck = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      _hasScheduledCheck = false;
      if (mounted) {
        _evaluateVisibility();
      }
    });
  }

  void _evaluateVisibility() {
    final String? token = widget.trackingToken;
    if (!_isForeground ||
        !_isRouteVisible ||
        token == null ||
        token.isEmpty ||
        token == _recordedToken) {
      _cancelDwell();
      return;
    }
    if (_visibleFraction() < widget.minimumVisibleFraction) {
      _cancelDwell();
      return;
    }
    if (_dwellTimer != null) {
      return;
    }
    _dwellTimer = Timer(widget.dwellDuration, () {
      _dwellTimer = null;
      if (!mounted ||
          !_isForeground ||
          !_isRouteVisible ||
          widget.trackingToken != token) {
        return;
      }
      if (_visibleFraction() < widget.minimumVisibleFraction) {
        return;
      }
      _recordedToken = token;
      widget.onImpression();
    });
  }

  void _cancelDwell() {
    _dwellTimer?.cancel();
    _dwellTimer = null;
  }

  void _handleTickerModeChanged() {
    _updateRouteVisibility();
  }

  void _updateRouteVisibility() {
    final bool isRouteVisible =
        _isScopeVisible && (_tickerMode?.value.enabled ?? false);
    if (isRouteVisible == _isRouteVisible) {
      return;
    }
    _isRouteVisible = isRouteVisible;
    if (isRouteVisible) {
      _scheduleVisibilityCheck();
    } else {
      _cancelDwell();
    }
  }

  double _visibleFraction() {
    final RenderObject? targetObject = context.findRenderObject();
    if (targetObject is! RenderBox ||
        !targetObject.hasSize ||
        targetObject.size.isEmpty) {
      return 0;
    }
    final Rect targetRect =
        targetObject.localToGlobal(Offset.zero) & targetObject.size;
    final RenderObject? viewportObject = _scrollable?.context
        .findRenderObject();
    final Rect viewportRect =
        viewportObject is RenderBox && viewportObject.hasSize
        ? viewportObject.localToGlobal(Offset.zero) & viewportObject.size
        : Offset.zero & MediaQuery.sizeOf(context);
    final Rect intersection = targetRect.intersect(viewportRect);
    if (intersection.isEmpty) {
      return 0;
    }
    return (intersection.width * intersection.height) /
        (targetRect.width * targetRect.height);
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
