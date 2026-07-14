//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'activity_day.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ActivityDay {
  /// Returns a new [ActivityDay] instance.
  ActivityDay({
    required this.date,

    required this.threads,

    required this.comments,

    required this.likes,

    required this.checkIns,

    required this.score,
  });

  @JsonKey(name: r'date', required: true, includeIfNull: false)
  final DateTime date;

  // minimum: 0
  @JsonKey(name: r'threads', required: true, includeIfNull: false)
  final int threads;

  // minimum: 0
  @JsonKey(name: r'comments', required: true, includeIfNull: false)
  final int comments;

  /// Positive likes given by the user.
  // minimum: 0
  @JsonKey(name: r'likes', required: true, includeIfNull: false)
  final int likes;

  /// Idempotent daily check-in recorded on the server's Asia/Shanghai calendar.
  // minimum: 0
  // maximum: 1
  @JsonKey(name: r'checkIns', required: true, includeIfNull: false)
  final int checkIns;

  // minimum: 0
  @JsonKey(name: r'score', required: true, includeIfNull: false)
  final int score;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ActivityDay &&
          other.date == date &&
          other.threads == threads &&
          other.comments == comments &&
          other.likes == likes &&
          other.checkIns == checkIns &&
          other.score == score;

  @override
  int get hashCode =>
      date.hashCode +
      threads.hashCode +
      comments.hashCode +
      likes.hashCode +
      checkIns.hashCode +
      score.hashCode;

  factory ActivityDay.fromJson(Map<String, dynamic> json) =>
      _$ActivityDayFromJson(json);

  Map<String, dynamic> toJson() => _$ActivityDayToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
