//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'search_highlight_range.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SearchHighlightRange {
  /// Returns a new [SearchHighlightRange] instance.
  SearchHighlightRange({required this.start, required this.end});

  // minimum: 0
  @JsonKey(name: r'start', required: true, includeIfNull: false)
  final int start;

  // minimum: 1
  @JsonKey(name: r'end', required: true, includeIfNull: false)
  final int end;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SearchHighlightRange && other.start == start && other.end == end;

  @override
  int get hashCode => start.hashCode + end.hashCode;

  factory SearchHighlightRange.fromJson(Map<String, dynamic> json) =>
      _$SearchHighlightRangeFromJson(json);

  Map<String, dynamic> toJson() => _$SearchHighlightRangeToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
