import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/search/data/search_repository.dart';
import 'package:yourtj_mobile/features/search/domain/search_models.dart';
import 'package:yourtj_mobile/features/search/presentation/search_page.dart';

void main() {
  testWidgets('account change reloads optional-auth search results', (
    WidgetTester tester,
  ) async {
    final StreamController<SessionState> sessions =
        StreamController<SessionState>();
    final _CountingSearchRepository repository = _CountingSearchRepository();
    addTearDown(sessions.close);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          federatedSearchRepositoryProvider.overrideWithValue(repository),
          sessionStateProvider.overrideWith((Ref ref) => sessions.stream),
        ],
        child: MaterialApp(
          theme: AppTheme.light,
          home: const SearchPage(
            initialQuery: '算法',
            initialScope: SearchScope.course,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final int anonymousCalls = repository.calls;

    sessions.add(
      SessionState.authenticated(generation: 1, account: _account('account-1')),
    );
    await tester.pumpAndSettle();
    final int firstAccountCalls = repository.calls;
    expect(firstAccountCalls, greaterThan(anonymousCalls));

    sessions.add(
      SessionState.authenticated(generation: 2, account: _account('account-2')),
    );
    await tester.pumpAndSettle();

    expect(repository.calls, greaterThan(firstAccountCalls));
    expect(find.text('搜索结果 ${repository.calls}'), findsOneWidget);
    expect(find.text('搜索结果 $firstAccountCalls'), findsNothing);
  });
}

class _CountingSearchRepository implements FederatedSearchRepository {
  int calls = 0;

  @override
  Future<SearchPageSlice> search({
    required String query,
    required SearchScope scope,
    required int limit,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    calls += 1;
    return SearchPageSlice(
      result: SearchResult(
        courses: <CourseSearchHit>[
          CourseSearchHit(
            id: 'course-$calls',
            code: 'CS$calls',
            name: '搜索结果 $calls',
            credit: 3,
            department: '计算机',
            teacherName: '张老师',
            reviewCount: 0,
            reviewAvg: 0,
          ),
        ],
        reviews: const <ReviewSearchHit>[],
        threads: const <ThreadSearchHit>[],
        users: const <UserSearchHit>[],
        boards: const <BoardSearchHit>[],
        tags: const <TagSearchHit>[],
        nextCursor: null,
        hasMore: false,
        hasMoreScopes: const <SearchResultScope>[],
        failedScopes: const <SearchResultScope>[],
        highlights: const <SearchHighlight>[],
        suggestedQuery: null,
      ),
    );
  }
}

Account _account(String id) {
  return Account(
    id: id,
    handle: id,
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: const <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 1,
  );
}
