//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'selection_offering.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SelectionOffering {
  /// Returns a new [SelectionOffering] instance.
  SelectionOffering({
    required this.id,

    required this.offeringId,

    required this.code,

    required this.teachingClassCode,

    required this.name,

    required this.credit,

    required this.natureId,

    required this.calendarId,

    required this.campusId,

    required this.facultyName,

    required this.teachingLanguage,

    required this.teacherName,

    required this.teacherNames,

    required this.startWeek,

    required this.endWeek,

    required this.weeksUnknown,

    required this.scheduleUnknown,

    required this.status,

    required this.catalogueCourseId,

    required this.reviewCount,

    required this.reviewAvg,

    required this.reviewScope,
  });

  /// Compatibility alias for offeringId.
  @Deprecated('id has been deprecated')
  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  /// Stable teachingClassId-backed identity used by schedules and detail routes.
  @JsonKey(name: r'offeringId', required: true, includeIfNull: false)
  final String offeringId;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @JsonKey(name: r'teachingClassCode', required: true, includeIfNull: true)
  final String? teachingClassCode;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'credit', required: true, includeIfNull: true)
  final num? credit;

  @JsonKey(name: r'natureId', required: true, includeIfNull: true)
  final String? natureId;

  @JsonKey(name: r'calendarId', required: true, includeIfNull: false)
  final String calendarId;

  @JsonKey(name: r'campusId', required: true, includeIfNull: true)
  final String? campusId;

  @JsonKey(name: r'facultyName', required: true, includeIfNull: true)
  final String? facultyName;

  @JsonKey(name: r'teachingLanguage', required: true, includeIfNull: true)
  final String? teachingLanguage;

  @JsonKey(name: r'teacherName', required: true, includeIfNull: true)
  final String? teacherName;

  @JsonKey(name: r'teacherNames', required: true, includeIfNull: false)
  final List<String> teacherNames;

  // minimum: 1
  // maximum: 30
  @JsonKey(name: r'startWeek', required: true, includeIfNull: true)
  final int? startWeek;

  // minimum: 1
  // maximum: 30
  @JsonKey(name: r'endWeek', required: true, includeIfNull: true)
  final int? endWeek;

  @JsonKey(name: r'weeksUnknown', required: true, includeIfNull: false)
  final bool weeksUnknown;

  /// True when a complete trustworthy schedule could not be materialized, including mixed parseable and unparseable arrangement input even if partial slots are available.
  @JsonKey(name: r'scheduleUnknown', required: true, includeIfNull: false)
  final bool scheduleUnknown;

  /// Upstream currently supplies no lifecycle status, so unknown is expected.
  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SelectionOfferingStatusEnum.unknownDefaultOpenApi,
  )
  final SelectionOfferingStatusEnum status;

  /// Optional deep-link target in the community course catalogue.
  @JsonKey(name: r'catalogueCourseId', required: true, includeIfNull: true)
  final String? catalogueCourseId;

  /// Public historical rating sample count associated with this teaching class.
  // minimum: 0
  @JsonKey(name: r'reviewCount', required: true, includeIfNull: false)
  final int reviewCount;

  /// Weighted historical rating, or null when reviewCount is zero.
  // minimum: 0
  // maximum: 5
  @JsonKey(name: r'reviewAvg', required: true, includeIfNull: true)
  final num? reviewAvg;

  /// Whether the historical aggregate matched the current teacher identity, fell back to a course-level alias, or has no data.
  @JsonKey(
    name: r'reviewScope',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SelectionOfferingReviewScopeEnum.unknownDefaultOpenApi,
  )
  final SelectionOfferingReviewScopeEnum reviewScope;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SelectionOffering &&
          other.id == id &&
          other.offeringId == offeringId &&
          other.code == code &&
          other.teachingClassCode == teachingClassCode &&
          other.name == name &&
          other.credit == credit &&
          other.natureId == natureId &&
          other.calendarId == calendarId &&
          other.campusId == campusId &&
          other.facultyName == facultyName &&
          other.teachingLanguage == teachingLanguage &&
          other.teacherName == teacherName &&
          other.teacherNames == teacherNames &&
          other.startWeek == startWeek &&
          other.endWeek == endWeek &&
          other.weeksUnknown == weeksUnknown &&
          other.scheduleUnknown == scheduleUnknown &&
          other.status == status &&
          other.catalogueCourseId == catalogueCourseId &&
          other.reviewCount == reviewCount &&
          other.reviewAvg == reviewAvg &&
          other.reviewScope == reviewScope;

  @override
  int get hashCode =>
      id.hashCode +
      offeringId.hashCode +
      code.hashCode +
      (teachingClassCode == null ? 0 : teachingClassCode.hashCode) +
      name.hashCode +
      (credit == null ? 0 : credit.hashCode) +
      (natureId == null ? 0 : natureId.hashCode) +
      calendarId.hashCode +
      (campusId == null ? 0 : campusId.hashCode) +
      (facultyName == null ? 0 : facultyName.hashCode) +
      (teachingLanguage == null ? 0 : teachingLanguage.hashCode) +
      (teacherName == null ? 0 : teacherName.hashCode) +
      teacherNames.hashCode +
      (startWeek == null ? 0 : startWeek.hashCode) +
      (endWeek == null ? 0 : endWeek.hashCode) +
      weeksUnknown.hashCode +
      scheduleUnknown.hashCode +
      status.hashCode +
      (catalogueCourseId == null ? 0 : catalogueCourseId.hashCode) +
      reviewCount.hashCode +
      (reviewAvg == null ? 0 : reviewAvg.hashCode) +
      reviewScope.hashCode;

  factory SelectionOffering.fromJson(Map<String, dynamic> json) =>
      _$SelectionOfferingFromJson(json);

  Map<String, dynamic> toJson() => _$SelectionOfferingToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

/// Upstream currently supplies no lifecycle status, so unknown is expected.
enum SelectionOfferingStatusEnum {
  /// Upstream currently supplies no lifecycle status, so unknown is expected.
  @JsonValue(r'unknown')
  unknown(r'unknown'),

  /// Upstream currently supplies no lifecycle status, so unknown is expected.
  @JsonValue(r'active')
  active(r'active'),

  /// Upstream currently supplies no lifecycle status, so unknown is expected.
  @JsonValue(r'cancelled')
  cancelled(r'cancelled'),

  /// Upstream currently supplies no lifecycle status, so unknown is expected.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SelectionOfferingStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

/// Whether the historical aggregate matched the current teacher identity, fell back to a course-level alias, or has no data.
enum SelectionOfferingReviewScopeEnum {
  /// Whether the historical aggregate matched the current teacher identity, fell back to a course-level alias, or has no data.
  @JsonValue(r'none')
  none(r'none'),

  /// Whether the historical aggregate matched the current teacher identity, fell back to a course-level alias, or has no data.
  @JsonValue(r'course')
  course(r'course'),

  /// Whether the historical aggregate matched the current teacher identity, fell back to a course-level alias, or has no data.
  @JsonValue(r'teacher')
  teacher(r'teacher'),

  /// Whether the historical aggregate matched the current teacher identity, fell back to a course-level alias, or has no data.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SelectionOfferingReviewScopeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
