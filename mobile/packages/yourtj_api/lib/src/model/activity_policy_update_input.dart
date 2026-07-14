//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/activity_weights.dart';
import 'package:json_annotation/json_annotation.dart';

part 'activity_policy_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ActivityPolicyUpdateInput {
  /// Returns a new [ActivityPolicyUpdateInput] instance.
  ActivityPolicyUpdateInput({
    required this.expectedVersion,

    required this.weights,

    required this.reason,
  });

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  @JsonKey(name: r'weights', required: true, includeIfNull: false)
  final ActivityWeights weights;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ActivityPolicyUpdateInput &&
          other.expectedVersion == expectedVersion &&
          other.weights == weights &&
          other.reason == reason;

  @override
  int get hashCode =>
      expectedVersion.hashCode + weights.hashCode + reason.hashCode;

  factory ActivityPolicyUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$ActivityPolicyUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$ActivityPolicyUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
