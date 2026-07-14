//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'appeal_transition_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AppealTransitionInput {
  /// Returns a new [AppealTransitionInput] instance.
  AppealTransitionInput({required this.expectedVersion, required this.reason});

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AppealTransitionInput &&
          other.expectedVersion == expectedVersion &&
          other.reason == reason;

  @override
  int get hashCode => expectedVersion.hashCode + reason.hashCode;

  factory AppealTransitionInput.fromJson(Map<String, dynamic> json) =>
      _$AppealTransitionInputFromJson(json);

  Map<String, dynamic> toJson() => _$AppealTransitionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
