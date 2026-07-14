import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/search/data/search_repository.dart';
import 'package:yourtj_mobile/features/search/domain/search_controller.dart';
import 'package:yourtj_mobile/features/search/domain/search_models.dart';

void main() {
  test(
    'preserves partial failures and deduplicates typed cursor pages',
    () async {
      final FederatedSearchController controller = FederatedSearchController(
        _FakeSearchRepository(),
        initialScope: SearchScope.course,
      );

      await controller.submit('算法', scope: SearchScope.course);
      expect(controller.courses.map((CourseSearchHit hit) => hit.id), <String>[
        '1',
      ]);
      expect(controller.failedScopes, contains(SearchResultScope.thread));
      expect(controller.hasMore, isTrue);
      expect(
        controller.rangesFor(
          scope: SearchResultScope.course,
          id: '1',
          field: SearchHighlightFieldEnum.name,
        ),
        hasLength(1),
      );

      await controller.loadMore();
      expect(controller.courses.map((CourseSearchHit hit) => hit.id), <String>[
        '1',
        '2',
      ]);
      expect(controller.failedScopes, contains(SearchResultScope.thread));
      expect(controller.hasMore, isFalse);

      controller.dispose();
    },
  );

  test('session invalidation rejects a late result before reloading', () async {
    final _DeferredSearchRepository repository = _DeferredSearchRepository();
    final FederatedSearchController controller = FederatedSearchController(
      repository,
      initialScope: SearchScope.course,
    );

    final Future<void> oldSearch = controller.submit(
      '算法',
      scope: SearchScope.course,
    );
    await Future<void>.delayed(Duration.zero);
    expect(repository.requests, hasLength(1));

    controller.invalidateForSessionChange();
    expect(controller.totalResults, 0);
    final Future<void> currentSearch = controller.reload();
    await Future<void>.delayed(Duration.zero);
    expect(repository.requests, hasLength(2));

    repository.requests.first.complete(_pageWithCourse('old'));
    await oldSearch;
    expect(controller.totalResults, 0);

    repository.requests.last.complete(_pageWithCourse('current'));
    await currentSearch;
    expect(controller.courses.single.id, 'current');

    controller.dispose();
  });
}

class _FakeSearchRepository implements FederatedSearchRepository {
  @override
  Future<SearchPageSlice> search({
    required String query,
    required SearchScope scope,
    required int limit,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    final bool next = cursor == 'next';
    return SearchPageSlice(
      result: SearchResult(
        courses: <CourseSearchHit>[
          if (!next) _course('1', '算法设计'),
          if (next) _course('1', '算法设计'),
          if (next) _course('2', '算法导论'),
        ],
        reviews: const <ReviewSearchHit>[],
        threads: const <ThreadSearchHit>[],
        users: const <UserSearchHit>[],
        boards: const <BoardSearchHit>[],
        tags: const <TagSearchHit>[],
        nextCursor: next ? null : 'next',
        hasMore: !next,
        hasMoreScopes: const <SearchResultScope>[],
        failedScopes: next
            ? const <SearchResultScope>[]
            : const <SearchResultScope>[SearchResultScope.thread],
        highlights: <SearchHighlight>[
          if (!next)
            SearchHighlight(
              scope: SearchResultScope.course,
              id: '1',
              field: SearchHighlightFieldEnum.name,
              ranges: <SearchHighlightRange>[
                SearchHighlightRange(start: 0, end: 2),
              ],
            ),
        ],
        suggestedQuery: null,
      ),
    );
  }
}

class _DeferredSearchRepository implements FederatedSearchRepository {
  final List<Completer<SearchPageSlice>> requests =
      <Completer<SearchPageSlice>>[];

  @override
  Future<SearchPageSlice> search({
    required String query,
    required SearchScope scope,
    required int limit,
    String? cursor,
    CancelToken? cancelToken,
  }) {
    final Completer<SearchPageSlice> request = Completer<SearchPageSlice>();
    requests.add(request);
    return request.future;
  }
}

SearchPageSlice _pageWithCourse(String id) {
  return SearchPageSlice(
    result: SearchResult(
      courses: <CourseSearchHit>[_course(id, '算法 $id')],
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

CourseSearchHit _course(String id, String name) {
  return CourseSearchHit(
    id: id,
    code: 'CS$id',
    name: name,
    credit: 3,
    department: '计算机',
    teacherName: '张老师',
    reviewCount: 2,
    reviewAvg: 4.5,
  );
}
