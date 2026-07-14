//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/notification_preferences.dart';
import 'package:json_annotation/json_annotation.dart';

part 'notification_prefs.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationPrefs {
  /// Returns a new [NotificationPrefs] instance.
  NotificationPrefs({required this.prefs});

  @JsonKey(name: r'prefs', required: true, includeIfNull: false)
  final NotificationPreferences prefs;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationPrefs && other.prefs == prefs;

  @override
  int get hashCode => prefs.hashCode;

  factory NotificationPrefs.fromJson(Map<String, dynamic> json) =>
      _$NotificationPrefsFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationPrefsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
