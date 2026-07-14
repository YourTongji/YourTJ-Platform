//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'report.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Report {
  /// Returns a new [Report] instance.
  Report({
    required this.id,

    required this.reviewId,

    required this.reason,

    required this.status,

    this.courseId,

    this.reviewAuthorHandle,

    this.reviewRating,

    this.reviewStatus,

    this.reviewExcerpt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'reviewId', required: true, includeIfNull: false)
  final String reviewId;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ReportStatusEnum.unknownDefaultOpenApi,
  )
  final ReportStatusEnum status;

  @JsonKey(name: r'courseId', required: false, includeIfNull: false)
  final String? courseId;

  @JsonKey(name: r'reviewAuthorHandle', required: false, includeIfNull: false)
  final String? reviewAuthorHandle;

  // minimum: 0
  // maximum: 5
  @JsonKey(name: r'reviewRating', required: false, includeIfNull: false)
  final int? reviewRating;

  @JsonKey(
    name: r'reviewStatus',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ReportReviewStatusEnum.unknownDefaultOpenApi,
  )
  final ReportReviewStatusEnum? reviewStatus;

  @JsonKey(name: r'reviewExcerpt', required: false, includeIfNull: false)
  final String? reviewExcerpt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Report &&
          other.id == id &&
          other.reviewId == reviewId &&
          other.reason == reason &&
          other.status == status &&
          other.courseId == courseId &&
          other.reviewAuthorHandle == reviewAuthorHandle &&
          other.reviewRating == reviewRating &&
          other.reviewStatus == reviewStatus &&
          other.reviewExcerpt == reviewExcerpt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      reviewId.hashCode +
      reason.hashCode +
      status.hashCode +
      (courseId == null ? 0 : courseId.hashCode) +
      (reviewAuthorHandle == null ? 0 : reviewAuthorHandle.hashCode) +
      (reviewRating == null ? 0 : reviewRating.hashCode) +
      (reviewStatus == null ? 0 : reviewStatus.hashCode) +
      (reviewExcerpt == null ? 0 : reviewExcerpt.hashCode) +
      createdAt.hashCode;

  factory Report.fromJson(Map<String, dynamic> json) => _$ReportFromJson(json);

  Map<String, dynamic> toJson() => _$ReportToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ReportStatusEnum {
  @JsonValue(r'open')
  open(r'open'),
  @JsonValue(r'upheld')
  upheld(r'upheld'),
  @JsonValue(r'rejected')
  rejected(r'rejected'),
  @JsonValue(r'ignored')
  ignored(r'ignored'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ReportStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum ReportReviewStatusEnum {
  @JsonValue(r'visible')
  visible(r'visible'),
  @JsonValue(r'hidden')
  hidden(r'hidden'),
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ReportReviewStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
