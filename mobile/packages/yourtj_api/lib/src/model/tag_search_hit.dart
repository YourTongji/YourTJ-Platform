//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'tag_search_hit.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TagSearchHit {
  /// Returns a new [TagSearchHit] instance.
  TagSearchHit({
    required this.id,

    required this.slug,

    required this.name,

    required this.description,

    required this.threadCount,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'description', required: true, includeIfNull: true)
  final String? description;

  // minimum: 0
  @JsonKey(name: r'threadCount', required: true, includeIfNull: false)
  final int threadCount;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TagSearchHit &&
          other.id == id &&
          other.slug == slug &&
          other.name == name &&
          other.description == description &&
          other.threadCount == threadCount;

  @override
  int get hashCode =>
      id.hashCode +
      slug.hashCode +
      name.hashCode +
      (description == null ? 0 : description.hashCode) +
      threadCount.hashCode;

  factory TagSearchHit.fromJson(Map<String, dynamic> json) =>
      _$TagSearchHitFromJson(json);

  Map<String, dynamic> toJson() => _$TagSearchHitToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
