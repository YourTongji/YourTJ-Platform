import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/reviews/presentation/review_card.dart';

void main() {
  testWidgets('owned review exposes edit without self-like or self-report', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        Review(
          id: 'review-1',
          rating: 5,
          authorHandle: 'alice',
          viewerLiked: false,
          canEdit: true,
          canReport: false,
        ),
      ),
    );

    expect(find.text('编辑'), findsOneWidget);
    expect(find.text('举报'), findsNothing);
    expect(find.textContaining('赞同'), findsNothing);
  });

  testWidgets('liked review exposes unlike state and report permission', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        Review(
          id: 'review-2',
          rating: 4,
          authorHandle: 'bob',
          approveCount: 3,
          viewerLiked: true,
          canEdit: false,
          canReport: true,
        ),
      ),
    );

    expect(find.text('3 已赞同'), findsOneWidget);
    expect(find.byIcon(Icons.favorite_rounded), findsOneWidget);
    expect(find.text('举报'), findsOneWidget);
  });
}

Widget _app(Review review) {
  return MaterialApp(
    home: Scaffold(
      body: ReviewCard(
        review: review,
        isBusy: false,
        onLike: () async {},
        onEdit: () async {},
        onReport: () async {},
        onRefreshAvatar: () {},
      ),
    ),
  );
}
