//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'appeal_decision_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AppealDecisionInput {
  /// Returns a new [AppealDecisionInput] instance.
  AppealDecisionInput({
    required this.expectedVersion,

    required this.outcome,

    required this.reason,

    this.amendedEndsAt,
  });

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  @JsonKey(
    name: r'outcome',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AppealDecisionInputOutcomeEnum.unknownDefaultOpenApi,
  )
  final AppealDecisionInputOutcomeEnum outcome;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  /// Only for amended sanctions; must shorten the active sanction.
  @JsonKey(name: r'amendedEndsAt', required: false, includeIfNull: false)
  final int? amendedEndsAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AppealDecisionInput &&
          other.expectedVersion == expectedVersion &&
          other.outcome == outcome &&
          other.reason == reason &&
          other.amendedEndsAt == amendedEndsAt;

  @override
  int get hashCode =>
      expectedVersion.hashCode +
      outcome.hashCode +
      reason.hashCode +
      (amendedEndsAt == null ? 0 : amendedEndsAt.hashCode);

  factory AppealDecisionInput.fromJson(Map<String, dynamic> json) =>
      _$AppealDecisionInputFromJson(json);

  Map<String, dynamic> toJson() => _$AppealDecisionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AppealDecisionInputOutcomeEnum {
  @JsonValue(r'upheld')
  upheld(r'upheld'),
  @JsonValue(r'overturned')
  overturned(r'overturned'),
  @JsonValue(r'amended')
  amended(r'amended'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AppealDecisionInputOutcomeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
