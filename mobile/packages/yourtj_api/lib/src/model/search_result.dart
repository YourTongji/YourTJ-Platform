//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/user_search_hit.dart';
import 'package:yourtj_api/src/model/search_result_scope.dart';
import 'package:yourtj_api/src/model/review_search_hit.dart';
import 'package:yourtj_api/src/model/tag_search_hit.dart';
import 'package:yourtj_api/src/model/course_search_hit.dart';
import 'package:yourtj_api/src/model/board_search_hit.dart';
import 'package:yourtj_api/src/model/thread_search_hit.dart';
import 'package:yourtj_api/src/model/search_highlight.dart';
import 'package:json_annotation/json_annotation.dart';

part 'search_result.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SearchResult {
  /// Returns a new [SearchResult] instance.
  SearchResult({
    required this.courses,

    required this.reviews,

    required this.threads,

    required this.users,

    required this.boards,

    required this.tags,

    required this.nextCursor,

    required this.hasMore,

    required this.hasMoreScopes,

    required this.failedScopes,

    required this.highlights,

    required this.suggestedQuery,
  });

  @JsonKey(name: r'courses', required: true, includeIfNull: false)
  final List<CourseSearchHit> courses;

  @JsonKey(name: r'reviews', required: true, includeIfNull: false)
  final List<ReviewSearchHit> reviews;

  @JsonKey(name: r'threads', required: true, includeIfNull: false)
  final List<ThreadSearchHit> threads;

  @JsonKey(name: r'users', required: true, includeIfNull: false)
  final List<UserSearchHit> users;

  @JsonKey(name: r'boards', required: true, includeIfNull: false)
  final List<BoardSearchHit> boards;

  @JsonKey(name: r'tags', required: true, includeIfNull: false)
  final List<TagSearchHit> tags;

  /// Opaque continuation for a specific non-all type, bound to the normalized query and type.
  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  /// Sections with more privacy-revalidated results; all-scope clients use this for \"see more\" links.
  @JsonKey(name: r'hasMoreScopes', required: true, includeIfNull: false)
  final List<SearchResultScope> hasMoreScopes;

  /// Included sections that failed while other sections remained usable. Never contains internal errors.
  @JsonKey(name: r'failedScopes', required: true, includeIfNull: false)
  final List<SearchResultScope> failedScopes;

  /// Safe ranges over the returned canonical fields; never snippets or markup copied from the search index.
  @JsonKey(name: r'highlights', required: true, includeIfNull: false)
  final List<SearchHighlight> highlights;

  /// Conservative spelling suggestion inferred only from words in the visible, owner-rehydrated result set; null when ambiguous.
  @JsonKey(name: r'suggestedQuery', required: true, includeIfNull: true)
  final String? suggestedQuery;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SearchResult &&
          other.courses == courses &&
          other.reviews == reviews &&
          other.threads == threads &&
          other.users == users &&
          other.boards == boards &&
          other.tags == tags &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore &&
          other.hasMoreScopes == hasMoreScopes &&
          other.failedScopes == failedScopes &&
          other.highlights == highlights &&
          other.suggestedQuery == suggestedQuery;

  @override
  int get hashCode =>
      courses.hashCode +
      reviews.hashCode +
      threads.hashCode +
      users.hashCode +
      boards.hashCode +
      tags.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode +
      hasMoreScopes.hashCode +
      failedScopes.hashCode +
      highlights.hashCode +
      (suggestedQuery == null ? 0 : suggestedQuery.hashCode);

  factory SearchResult.fromJson(Map<String, dynamic> json) =>
      _$SearchResultFromJson(json);

  Map<String, dynamic> toJson() => _$SearchResultToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
