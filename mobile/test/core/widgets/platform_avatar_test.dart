import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/config/app_environment.dart';
import 'package:yourtj_mobile/core/widgets/platform_avatar.dart';

void main() {
  testWidgets('prefers a current typed delivery over a compatibility URL', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        PlatformAvatar(
          delivery: _delivery(
            url: 'https://media.yourtj.de/avatar.webp',
            expiresAt: DateTime.now().millisecondsSinceEpoch ~/ 1000 + 5 * 60,
          ),
          compatibilityUrl: 'https://legacy.yourtj.de/avatar.webp',
          fallbackText: 'alice',
        ),
      ),
    );

    final CircleAvatar avatar = tester.widget<CircleAvatar>(
      find.byType(CircleAvatar),
    );
    expect(
      (avatar.foregroundImage! as NetworkImage).url,
      'https://media.yourtj.de/avatar.webp',
    );
  });

  testWidgets('expired typed delivery falls back and refreshes only once', (
    WidgetTester tester,
  ) async {
    int refreshes = 0;
    await tester.pumpWidget(
      _app(
        PlatformAvatar(
          delivery: _delivery(
            url: 'https://media.yourtj.de/avatar.webp',
            expiresAt: DateTime.now().millisecondsSinceEpoch ~/ 1000 - 1,
          ),
          compatibilityUrl: 'https://legacy.yourtj.de/avatar.webp',
          fallbackText: 'alice',
          onRefresh: () => refreshes += 1,
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    final CircleAvatar avatar = tester.widget<CircleAvatar>(
      find.byType(CircleAvatar),
    );
    expect(avatar.foregroundImage, isNull);
    expect(find.text('A'), findsOneWidget);
    expect(refreshes, 1);
  });

  testWidgets('rejects unsafe compatibility URL and requests a refresh', (
    WidgetTester tester,
  ) async {
    int refreshes = 0;
    await tester.pumpWidget(
      _app(
        PlatformAvatar(
          compatibilityUrl: 'http://media.yourtj.de/avatar.webp',
          fallbackText: '',
          onRefresh: () => refreshes += 1,
        ),
      ),
    );
    await tester.pump();

    final CircleAvatar avatar = tester.widget<CircleAvatar>(
      find.byType(CircleAvatar),
    );
    expect(avatar.foregroundImage, isNull);
    expect(find.text('?'), findsOneWidget);
    expect(refreshes, 1);
  });

  testWidgets('network failure requests a refresh only once per source', (
    WidgetTester tester,
  ) async {
    int refreshes = 0;
    await tester.pumpWidget(
      _app(
        PlatformAvatar(
          compatibilityUrl: 'https://media.yourtj.de/avatar.webp',
          fallbackText: 'alice',
          onRefresh: () => refreshes += 1,
        ),
      ),
    );

    final CircleAvatar avatar = tester.widget<CircleAvatar>(
      find.byType(CircleAvatar),
    );
    avatar.onForegroundImageError!(StateError('first failure'), null);
    avatar.onForegroundImageError!(StateError('second failure'), null);
    await tester.pump();

    expect(refreshes, 1);
  });

  testWidgets('rejects a third-party HTTPS avatar outside configured origins', (
    WidgetTester tester,
  ) async {
    int refreshes = 0;
    await tester.pumpWidget(
      _app(
        PlatformAvatar(
          compatibilityUrl:
              'https://tracker.example/collect/avatar.webp?account=secret',
          fallbackText: 'alice',
          onRefresh: () => refreshes += 1,
        ),
      ),
    );
    await tester.pump();

    final CircleAvatar avatar = tester.widget<CircleAvatar>(
      find.byType(CircleAvatar),
    );
    expect(avatar.foregroundImage, isNull);
    expect(find.text('A'), findsOneWidget);
    expect(refreshes, 1);
  });

  testWidgets('platform image rejects unsafe URL and refreshes once', (
    WidgetTester tester,
  ) async {
    int refreshes = 0;
    await tester.pumpWidget(
      _app(
        PlatformImage(
          url: 'https://user@media.yourtj.de/banner.webp',
          height: 160,
          semanticLabel: '个人主页封面',
          onRefresh: () => refreshes += 1,
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(Image), findsNothing);
    expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
    expect(refreshes, 1);
  });
}

Widget _app(Widget child) => PlatformMediaScope(
  environment: AppEnvironment(
    apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
    mediaCdnBaseUri: Uri.parse('https://media.yourtj.de'),
  ),
  child: MaterialApp(home: Scaffold(body: child)),
);

MediaDelivery _delivery({required String url, required int expiresAt}) {
  return MediaDelivery(
    assetId: 'asset-1',
    variant: MediaDeliveryVariant.thumb256,
    url: url,
    expiresAt: expiresAt,
    mime: MediaDeliveryMimeEnum.imageSlashWebp,
    width: 256,
    height: 256,
  );
}
