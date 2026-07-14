//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'review_search_hit.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ReviewSearchHit {
  /// Returns a new [ReviewSearchHit] instance.
  ReviewSearchHit({
    required this.id,

    required this.courseId,

    required this.courseName,

    required this.rating,

    required this.comment,

    required this.approveCount,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'courseId', required: true, includeIfNull: false)
  final String courseId;

  @JsonKey(name: r'courseName', required: true, includeIfNull: false)
  final String courseName;

  // minimum: 0
  // maximum: 5
  @JsonKey(name: r'rating', required: true, includeIfNull: false)
  final int rating;

  @JsonKey(name: r'comment', required: true, includeIfNull: true)
  final String? comment;

  // minimum: 0
  @JsonKey(name: r'approveCount', required: true, includeIfNull: false)
  final int approveCount;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ReviewSearchHit &&
          other.id == id &&
          other.courseId == courseId &&
          other.courseName == courseName &&
          other.rating == rating &&
          other.comment == comment &&
          other.approveCount == approveCount &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      courseId.hashCode +
      courseName.hashCode +
      rating.hashCode +
      (comment == null ? 0 : comment.hashCode) +
      approveCount.hashCode +
      createdAt.hashCode;

  factory ReviewSearchHit.fromJson(Map<String, dynamic> json) =>
      _$ReviewSearchHitFromJson(json);

  Map<String, dynamic> toJson() => _$ReviewSearchHitToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
