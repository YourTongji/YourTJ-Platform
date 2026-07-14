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
    required this.courseId,

    required this.teacherName,

    required this.weekday,

    required this.startSlot,

    required this.endSlot,

    required this.weeks,

    required this.location,
  });

  @JsonKey(name: r'courseId', required: true, includeIfNull: false)
  final String courseId;

  @JsonKey(name: r'teacherName', required: true, includeIfNull: true)
  final String? teacherName;

  @JsonKey(name: r'weekday', required: true, includeIfNull: false)
  final int weekday;

  @JsonKey(name: r'startSlot', required: true, includeIfNull: false)
  final int startSlot;

  @JsonKey(name: r'endSlot', required: true, includeIfNull: false)
  final int endSlot;

  @JsonKey(name: r'weeks', required: true, includeIfNull: true)
  final String? weeks;

  @JsonKey(name: r'location', required: true, includeIfNull: true)
  final String? location;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TimeSlot &&
          other.courseId == courseId &&
          other.teacherName == teacherName &&
          other.weekday == weekday &&
          other.startSlot == startSlot &&
          other.endSlot == endSlot &&
          other.weeks == weeks &&
          other.location == location;

  @override
  int get hashCode =>
      courseId.hashCode +
      (teacherName == null ? 0 : teacherName.hashCode) +
      weekday.hashCode +
      startSlot.hashCode +
      endSlot.hashCode +
      (weeks == null ? 0 : weeks.hashCode) +
      (location == null ? 0 : location.hashCode);

  factory TimeSlot.fromJson(Map<String, dynamic> json) =>
      _$TimeSlotFromJson(json);

  Map<String, dynamic> toJson() => _$TimeSlotToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
