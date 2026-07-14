//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'email_notification_prefs.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class EmailNotificationPrefs {
  /// Returns a new [EmailNotificationPrefs] instance.
  EmailNotificationPrefs({this.weeklyDigest = false});

  @JsonKey(
    defaultValue: false,
    name: r'weeklyDigest',
    required: true,
    includeIfNull: false,
  )
  final bool weeklyDigest;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is EmailNotificationPrefs && other.weeklyDigest == weeklyDigest;

  @override
  int get hashCode => weeklyDigest.hashCode;

  factory EmailNotificationPrefs.fromJson(Map<String, dynamic> json) =>
      _$EmailNotificationPrefsFromJson(json);

  Map<String, dynamic> toJson() => _$EmailNotificationPrefsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
