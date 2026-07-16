//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'time_slot.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TimeSlot {
  /// Returns a new [TimeSlot] instance.
  TimeSlot({
    required this.offeringId,

    required this.courseId,

    required this.teacherName,

    required this.weekday,

    required this.startSlot,

    required this.endSlot,

    required this.weeks,

    required this.weekNumbers,

    required this.weeksUnknown,

    required this.location,

    required this.locationUnknown,
  });

  @JsonKey(name: r'offeringId', required: true, includeIfNull: false)
  final String offeringId;

  /// Compatibility alias for offeringId.
  @Deprecated('courseId has been deprecated')
  @JsonKey(name: r'courseId', required: true, includeIfNull: false)
  final String courseId;

  @JsonKey(name: r'teacherName', required: true, includeIfNull: true)
  final String? teacherName;

  // minimum: 1
  // maximum: 7
  @JsonKey(name: r'weekday', required: true, includeIfNull: false)
  final int weekday;

  // minimum: 1
  // maximum: 20
  @JsonKey(name: r'startSlot', required: true, includeIfNull: false)
  final int startSlot;

  // minimum: 1
  // maximum: 20
  @JsonKey(name: r'endSlot', required: true, includeIfNull: false)
  final int endSlot;

  @JsonKey(name: r'weeks', required: true, includeIfNull: true)
  final String? weeks;

  @JsonKey(name: r'weekNumbers', required: true, includeIfNull: false)
  final Set<int> weekNumbers;

  @JsonKey(name: r'weeksUnknown', required: true, includeIfNull: false)
  final bool weeksUnknown;

  @JsonKey(name: r'location', required: true, includeIfNull: true)
  final String? location;

  @JsonKey(name: r'locationUnknown', required: true, includeIfNull: false)
  final bool locationUnknown;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TimeSlot &&
          other.offeringId == offeringId &&
          other.courseId == courseId &&
          other.teacherName == teacherName &&
          other.weekday == weekday &&
          other.startSlot == startSlot &&
          other.endSlot == endSlot &&
          other.weeks == weeks &&
          other.weekNumbers == weekNumbers &&
          other.weeksUnknown == weeksUnknown &&
          other.location == location &&
          other.locationUnknown == locationUnknown;

  @override
  int get hashCode =>
      offeringId.hashCode +
      courseId.hashCode +
      (teacherName == null ? 0 : teacherName.hashCode) +
      weekday.hashCode +
      startSlot.hashCode +
      endSlot.hashCode +
      (weeks == null ? 0 : weeks.hashCode) +
      weekNumbers.hashCode +
      weeksUnknown.hashCode +
      (location == null ? 0 : location.hashCode) +
      locationUnknown.hashCode;

  factory TimeSlot.fromJson(Map<String, dynamic> json) =>
      _$TimeSlotFromJson(json);

  Map<String, dynamic> toJson() => _$TimeSlotToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
