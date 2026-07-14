import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

enum SearchScope { all, course, review, thread, user, board, tag }

extension SearchScopeMetadata on SearchScope {
  String get wireValue => name;

  String get label => switch (this) {
    SearchScope.all => '全部',
    SearchScope.course => '课程与教师',
    SearchScope.review => '课评',
    SearchScope.thread => '社区帖子',
    SearchScope.user => '用户',
    SearchScope.board => '板块',
    SearchScope.tag => '标签',
  };

  SearchResultScope? get generatedScope => switch (this) {
    SearchScope.all => null,
    SearchScope.course => SearchResultScope.course,
    SearchScope.review => SearchResultScope.review,
    SearchScope.thread => SearchResultScope.thread,
    SearchScope.user => SearchResultScope.user,
    SearchScope.board => SearchResultScope.board,
    SearchScope.tag => SearchResultScope.tag,
  };
}

SearchScope searchScopeFromWire(String? value) {
  return SearchScope.values.firstWhere(
    (SearchScope scope) => scope.wireValue == value,
    orElse: () => SearchScope.all,
  );
}

@immutable
class SearchPageSlice {
  const SearchPageSlice({required this.result});

  final SearchResult result;
}
