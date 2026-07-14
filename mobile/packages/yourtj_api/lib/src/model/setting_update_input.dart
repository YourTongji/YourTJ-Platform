//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'setting_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SettingUpdateInput {
  /// Returns a new [SettingUpdateInput] instance.
  SettingUpdateInput({required this.value, required this.reason});

  @JsonKey(name: r'value', required: true, includeIfNull: false)
  final String value;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SettingUpdateInput &&
          other.value == value &&
          other.reason == reason;

  @override
  int get hashCode => value.hashCode + reason.hashCode;

  factory SettingUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$SettingUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$SettingUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
