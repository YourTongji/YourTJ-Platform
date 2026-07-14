//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/email_notification_prefs.dart';
import 'package:yourtj_api/src/model/in_app_notification_prefs.dart';
import 'package:json_annotation/json_annotation.dart';

part 'notification_preferences.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationPreferences {
  /// Returns a new [NotificationPreferences] instance.
  NotificationPreferences({required this.inApp, required this.email});

  @JsonKey(name: r'inApp', required: true, includeIfNull: false)
  final InAppNotificationPrefs inApp;

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final EmailNotificationPrefs email;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationPreferences &&
          other.inApp == inApp &&
          other.email == email;

  @override
  int get hashCode => inApp.hashCode + email.hashCode;

  factory NotificationPreferences.fromJson(Map<String, dynamic> json) =>
      _$NotificationPreferencesFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationPreferencesToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
