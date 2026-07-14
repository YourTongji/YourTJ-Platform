//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/search_result_scope.dart';
import 'package:yourtj_api/src/model/search_highlight_range.dart';
import 'package:json_annotation/json_annotation.dart';

part 'search_highlight.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SearchHighlight {
  /// Returns a new [SearchHighlight] instance.
  SearchHighlight({
    required this.scope,

    required this.id,

    required this.field,

    required this.ranges,
  });

  @JsonKey(
    name: r'scope',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SearchResultScope.unknownDefaultOpenApi,
  )
  final SearchResultScope scope;

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'field',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SearchHighlightFieldEnum.unknownDefaultOpenApi,
  )
  final SearchHighlightFieldEnum field;

  @JsonKey(name: r'ranges', required: true, includeIfNull: false)
  final List<SearchHighlightRange> ranges;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SearchHighlight &&
          other.scope == scope &&
          other.id == id &&
          other.field == field &&
          other.ranges == ranges;

  @override
  int get hashCode =>
      scope.hashCode + id.hashCode + field.hashCode + ranges.hashCode;

  factory SearchHighlight.fromJson(Map<String, dynamic> json) =>
      _$SearchHighlightFromJson(json);

  Map<String, dynamic> toJson() => _$SearchHighlightToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum SearchHighlightFieldEnum {
  @JsonValue(r'name')
  name(r'name'),
  @JsonValue(r'code')
  code(r'code'),
  @JsonValue(r'teacherName')
  teacherName(r'teacherName'),
  @JsonValue(r'department')
  department(r'department'),
  @JsonValue(r'courseName')
  courseName(r'courseName'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'title')
  title(r'title'),
  @JsonValue(r'bodyExcerpt')
  bodyExcerpt(r'bodyExcerpt'),
  @JsonValue(r'board')
  board(r'board'),
  @JsonValue(r'authorHandle')
  authorHandle(r'authorHandle'),
  @JsonValue(r'handle')
  handle(r'handle'),
  @JsonValue(r'displayName')
  displayName(r'displayName'),
  @JsonValue(r'slug')
  slug(r'slug'),
  @JsonValue(r'description')
  description(r'description'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SearchHighlightFieldEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
