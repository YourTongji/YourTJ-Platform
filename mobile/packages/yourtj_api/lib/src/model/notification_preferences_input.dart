//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/in_app_notification_prefs_input.dart';
import 'package:yourtj_api/src/model/email_notification_prefs.dart';
import 'package:json_annotation/json_annotation.dart';

part 'notification_preferences_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationPreferencesInput {
  /// Returns a new [NotificationPreferencesInput] instance.
  NotificationPreferencesInput({required this.inApp, required this.email});

  @JsonKey(name: r'inApp', required: true, includeIfNull: false)
  final InAppNotificationPrefsInput inApp;

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final EmailNotificationPrefs email;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationPreferencesInput &&
          other.inApp == inApp &&
          other.email == email;

  @override
  int get hashCode => inApp.hashCode + email.hashCode;

  factory NotificationPreferencesInput.fromJson(Map<String, dynamic> json) =>
      _$NotificationPreferencesInputFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationPreferencesInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
