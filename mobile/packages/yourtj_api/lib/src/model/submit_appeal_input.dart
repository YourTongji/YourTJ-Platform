//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'submit_appeal_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SubmitAppealInput {
  /// Returns a new [SubmitAppealInput] instance.
  SubmitAppealInput({required this.governanceEventId, required this.reason});

  @JsonKey(name: r'governanceEventId', required: true, includeIfNull: false)
  final String governanceEventId;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SubmitAppealInput &&
          other.governanceEventId == governanceEventId &&
          other.reason == reason;

  @override
  int get hashCode => governanceEventId.hashCode + reason.hashCode;

  factory SubmitAppealInput.fromJson(Map<String, dynamic> json) =>
      _$SubmitAppealInputFromJson(json);

  Map<String, dynamic> toJson() => _$SubmitAppealInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
