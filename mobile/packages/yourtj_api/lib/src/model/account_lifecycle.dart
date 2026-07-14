//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/account_lifecycle_state.dart';
import 'package:json_annotation/json_annotation.dart';

part 'account_lifecycle.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AccountLifecycle {
  /// Returns a new [AccountLifecycle] instance.
  AccountLifecycle({
    required this.state,

    required this.deactivatedAt,

    required this.deletionRequestedAt,

    required this.recoverUntil,

    required this.deletedAt,

    required this.purgedAt,

    required this.lifecycleVersion,
  });

  @JsonKey(
    name: r'state',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AccountLifecycleState.unknownDefaultOpenApi,
  )
  final AccountLifecycleState state;

  @JsonKey(name: r'deactivatedAt', required: true, includeIfNull: true)
  final int? deactivatedAt;

  @JsonKey(name: r'deletionRequestedAt', required: true, includeIfNull: true)
  final int? deletionRequestedAt;

  @JsonKey(name: r'recoverUntil', required: true, includeIfNull: true)
  final int? recoverUntil;

  @JsonKey(name: r'deletedAt', required: true, includeIfNull: true)
  final int? deletedAt;

  @JsonKey(name: r'purgedAt', required: true, includeIfNull: true)
  final int? purgedAt;

  // minimum: 1
  @JsonKey(name: r'lifecycleVersion', required: true, includeIfNull: false)
  final int lifecycleVersion;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AccountLifecycle &&
          other.state == state &&
          other.deactivatedAt == deactivatedAt &&
          other.deletionRequestedAt == deletionRequestedAt &&
          other.recoverUntil == recoverUntil &&
          other.deletedAt == deletedAt &&
          other.purgedAt == purgedAt &&
          other.lifecycleVersion == lifecycleVersion;

  @override
  int get hashCode =>
      state.hashCode +
      (deactivatedAt == null ? 0 : deactivatedAt.hashCode) +
      (deletionRequestedAt == null ? 0 : deletionRequestedAt.hashCode) +
      (recoverUntil == null ? 0 : recoverUntil.hashCode) +
      (deletedAt == null ? 0 : deletedAt.hashCode) +
      (purgedAt == null ? 0 : purgedAt.hashCode) +
      lifecycleVersion.hashCode;

  factory AccountLifecycle.fromJson(Map<String, dynamic> json) =>
      _$AccountLifecycleFromJson(json);

  Map<String, dynamic> toJson() => _$AccountLifecycleToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
