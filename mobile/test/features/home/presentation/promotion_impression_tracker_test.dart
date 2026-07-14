import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/navigation/app_route_visibility.dart';
import 'package:yourtj_mobile/features/home/presentation/promotion_impression_tracker.dart';

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized()
        .handleAppLifecycleStateChanged(AppLifecycleState.resumed);
  });

  testWidgets(
    'requires exactly half visibility for five hundred milliseconds',
    (WidgetTester tester) async {
      final ScrollController controller = ScrollController();
      int impressions = 0;
      await tester.pumpWidget(
        _TrackerHarness(
          controller: controller,
          onImpression: () => impressions += 1,
        ),
      );

      controller.jumpTo(98);
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(impressions, 0);

      controller.jumpTo(100);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 499));
      expect(impressions, 0);
      await tester.pump(const Duration(milliseconds: 1));
      expect(impressions, 1);

      await tester.pump(const Duration(seconds: 1));
      expect(impressions, 1);
    },
  );

  testWidgets('visibility loss cancels and restarts the dwell window', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    int impressions = 0;
    await tester.pumpWidget(
      _TrackerHarness(
        controller: controller,
        onImpression: () => impressions += 1,
      ),
    );

    controller.jumpTo(500);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    controller.jumpTo(0);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(impressions, 0);

    controller.jumpTo(500);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 499));
    expect(impressions, 0);
    await tester.pump(const Duration(milliseconds: 1));
    expect(impressions, 1);
  });

  testWidgets('backgrounding cancels the active dwell window', (
    WidgetTester tester,
  ) async {
    final ScrollController controller = ScrollController();
    int impressions = 0;
    await tester.pumpWidget(
      _TrackerHarness(
        controller: controller,
        onImpression: () => impressions += 1,
      ),
    );

    controller.jumpTo(500);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump(const Duration(milliseconds: 300));
    expect(impressions, 0);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 499));
    expect(impressions, 0);
    await tester.pump(const Duration(milliseconds: 1));
    expect(impressions, 1);
  });

  testWidgets('creation while paused stays fail-closed until resumed', (
    WidgetTester tester,
  ) async {
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    addTearDown(
      () => tester.binding.handleAppLifecycleStateChanged(
        AppLifecycleState.resumed,
      ),
    );
    final ScrollController controller = ScrollController(
      initialScrollOffset: 500,
    );
    int impressions = 0;

    await tester.pumpWidget(
      _TrackerHarness(
        controller: controller,
        onImpression: () => impressions += 1,
      ),
    );
    await tester.pump(const Duration(seconds: 1));
    expect(impressions, 0);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 499));
    expect(impressions, 0);
    await tester.pump(const Duration(milliseconds: 1));
    expect(impressions, 1);
  });

  testWidgets('offstage branch cancels and restarts the dwell window', (
    WidgetTester tester,
  ) async {
    final ValueNotifier<bool> isActive = ValueNotifier<bool>(true);
    addTearDown(isActive.dispose);
    int impressions = 0;

    await tester.pumpWidget(
      _OffstageHarness(
        isActive: isActive,
        onImpression: () => impressions += 1,
      ),
    );
    await tester.pump(const Duration(milliseconds: 300));
    isActive.value = false;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(
      TickerMode.valuesOf(
        tester.element(
          find.byType(PromotionImpressionTracker, skipOffstage: false),
        ),
      ).enabled,
      isFalse,
    );
    expect(impressions, 0);

    isActive.value = true;
    await tester.pump();
    // TickerMode publishes its effective inherited value on the next frame.
    await tester.pump();
    expect(
      TickerMode.valuesOf(
        tester.element(find.byType(PromotionImpressionTracker)),
      ).enabled,
      isTrue,
    );
    await tester.pump(const Duration(milliseconds: 499));
    expect(impressions, 0);
    await tester.pump(const Duration(milliseconds: 1));
    expect(impressions, 1);
  });

  testWidgets('covering root route cancels the active dwell window', (
    WidgetTester tester,
  ) async {
    final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();
    int impressions = 0;
    await tester.pumpWidget(
      MaterialApp(
        navigatorKey: navigatorKey,
        home: _RouteTrackedPage(onImpression: () => impressions += 1),
      ),
    );
    await tester.pump(const Duration(milliseconds: 300));

    unawaited(
      navigatorKey.currentState!.push(
        MaterialPageRoute<void>(
          builder: (BuildContext context) =>
              const Scaffold(body: Center(child: Text('覆盖页面'))),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(impressions, 0);
  });
}

class _TrackerHarness extends StatelessWidget {
  const _TrackerHarness({required this.controller, required this.onImpression});

  final ScrollController controller;
  final VoidCallback onImpression;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: AppRouteVisibilityScope(
        isVisible: true,
        child: Scaffold(
          body: ListView(
            controller: controller,
            children: <Widget>[
              const SizedBox(height: 600),
              PromotionImpressionTracker(
                trackingToken: 'promotion-token',
                onImpression: onImpression,
                child: const SizedBox(height: 200),
              ),
              const SizedBox(height: 600),
            ],
          ),
        ),
      ),
    );
  }
}

class _OffstageHarness extends StatelessWidget {
  const _OffstageHarness({required this.isActive, required this.onImpression});

  final ValueListenable<bool> isActive;
  final VoidCallback onImpression;

  @override
  Widget build(BuildContext context) {
    final Widget tracker = PromotionImpressionTracker(
      trackingToken: 'promotion-token',
      onImpression: onImpression,
      child: const SizedBox(height: 200),
    );

    return MaterialApp(
      home: AppRouteVisibilityScope(
        isVisible: true,
        child: Scaffold(
          body: ListView(
            children: <Widget>[
              ValueListenableBuilder<bool>(
                valueListenable: isActive,
                child: tracker,
                builder: (BuildContext context, bool isActive, Widget? child) {
                  return Offstage(
                    offstage: !isActive,
                    child: TickerMode(enabled: isActive, child: child!),
                  );
                },
              ),
              const SizedBox(height: 600),
            ],
          ),
        ),
      ),
    );
  }
}

class _RouteTrackedPage extends StatelessWidget {
  const _RouteTrackedPage({required this.onImpression});

  final VoidCallback onImpression;

  @override
  Widget build(BuildContext context) {
    return AppRouteVisibilityScope(
      isVisible: ModalRoute.isCurrentOf(context) ?? false,
      child: Scaffold(
        body: PromotionImpressionTracker(
          trackingToken: 'promotion-token',
          onImpression: onImpression,
          child: const SizedBox(height: 200),
        ),
      ),
    );
  }
}
