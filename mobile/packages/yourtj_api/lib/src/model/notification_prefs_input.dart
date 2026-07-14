//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/notification_preferences_input.dart';
import 'package:json_annotation/json_annotation.dart';

part 'notification_prefs_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationPrefsInput {
  /// Returns a new [NotificationPrefsInput] instance.
  NotificationPrefsInput({required this.prefs});

  @JsonKey(name: r'prefs', required: true, includeIfNull: false)
  final NotificationPreferencesInput prefs;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationPrefsInput && other.prefs == prefs;

  @override
  int get hashCode => prefs.hashCode;

  factory NotificationPrefsInput.fromJson(Map<String, dynamic> json) =>
      _$NotificationPrefsInputFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationPrefsInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
