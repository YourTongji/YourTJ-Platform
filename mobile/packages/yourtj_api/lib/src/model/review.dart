//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'review.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Review {
  /// Returns a new [Review] instance.
  Review({
    this.id,

    this.courseId,

    this.rating,

    this.comment,

    this.score,

    this.semester,

    this.authorHandle,

    this.authorAvatar,

    this.approveCount,

    this.status,

    this.createdAt,
  });

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'courseId', required: false, includeIfNull: false)
  final String? courseId;

  @JsonKey(name: r'rating', required: false, includeIfNull: false)
  final int? rating;

  @JsonKey(name: r'comment', required: false, includeIfNull: false)
  final String? comment;

  @JsonKey(name: r'score', required: false, includeIfNull: false)
  final String? score;

  @JsonKey(name: r'semester', required: false, includeIfNull: false)
  final String? semester;

  @JsonKey(name: r'authorHandle', required: false, includeIfNull: false)
  final String? authorHandle;

  /// Reserved for a current platform-owned avatar projection. Legacy reviewer-provided remote URLs are never returned; null until Reviews integrates a typed Media projection.
  @JsonKey(name: r'authorAvatar', required: false, includeIfNull: false)
  final String? authorAvatar;

  @JsonKey(name: r'approveCount', required: false, includeIfNull: false)
  final int? approveCount;

  @JsonKey(
    name: r'status',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ReviewStatusEnum.unknownDefaultOpenApi,
  )
  final ReviewStatusEnum? status;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Review &&
          other.id == id &&
          other.courseId == courseId &&
          other.rating == rating &&
          other.comment == comment &&
          other.score == score &&
          other.semester == semester &&
          other.authorHandle == authorHandle &&
          other.authorAvatar == authorAvatar &&
          other.approveCount == approveCount &&
          other.status == status &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      courseId.hashCode +
      rating.hashCode +
      (comment == null ? 0 : comment.hashCode) +
      (score == null ? 0 : score.hashCode) +
      (semester == null ? 0 : semester.hashCode) +
      authorHandle.hashCode +
      (authorAvatar == null ? 0 : authorAvatar.hashCode) +
      approveCount.hashCode +
      status.hashCode +
      createdAt.hashCode;

  factory Review.fromJson(Map<String, dynamic> json) => _$ReviewFromJson(json);

  Map<String, dynamic> toJson() => _$ReviewToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ReviewStatusEnum {
  @JsonValue(r'visible')
  visible(r'visible'),
  @JsonValue(r'hidden')
  hidden(r'hidden'),
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ReviewStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
