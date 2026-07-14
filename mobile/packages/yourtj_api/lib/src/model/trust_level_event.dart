//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'trust_level_event.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TrustLevelEvent {
  /// Returns a new [TrustLevelEvent] instance.
  TrustLevelEvent({
    required this.id,

    required this.accountId,

    required this.eventKind,

    required this.fromLevel,

    required this.toLevel,

    required this.qualifyingScore,

    required this.policyVersion,

    required this.actorKind,

    required this.actorAccountId,

    required this.reason,

    required this.governanceEventId,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(
    name: r'eventKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: TrustLevelEventEventKindEnum.unknownDefaultOpenApi,
  )
  final TrustLevelEventEventKindEnum eventKind;

  // minimum: 0
  // maximum: 6
  @JsonKey(name: r'fromLevel', required: true, includeIfNull: false)
  final int fromLevel;

  // minimum: 1
  // maximum: 6
  @JsonKey(name: r'toLevel', required: true, includeIfNull: false)
  final int toLevel;

  // minimum: 0
  @JsonKey(name: r'qualifyingScore', required: true, includeIfNull: false)
  final int qualifyingScore;

  // minimum: 1
  @JsonKey(name: r'policyVersion', required: true, includeIfNull: false)
  final int policyVersion;

  @JsonKey(
    name: r'actorKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: TrustLevelEventActorKindEnum.unknownDefaultOpenApi,
  )
  final TrustLevelEventActorKindEnum actorKind;

  @JsonKey(name: r'actorAccountId', required: true, includeIfNull: true)
  final String? actorAccountId;

  @JsonKey(name: r'reason', required: true, includeIfNull: true)
  final String? reason;

  @JsonKey(name: r'governanceEventId', required: true, includeIfNull: true)
  final String? governanceEventId;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrustLevelEvent &&
          other.id == id &&
          other.accountId == accountId &&
          other.eventKind == eventKind &&
          other.fromLevel == fromLevel &&
          other.toLevel == toLevel &&
          other.qualifyingScore == qualifyingScore &&
          other.policyVersion == policyVersion &&
          other.actorKind == actorKind &&
          other.actorAccountId == actorAccountId &&
          other.reason == reason &&
          other.governanceEventId == governanceEventId &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      accountId.hashCode +
      eventKind.hashCode +
      fromLevel.hashCode +
      toLevel.hashCode +
      qualifyingScore.hashCode +
      policyVersion.hashCode +
      actorKind.hashCode +
      (actorAccountId == null ? 0 : actorAccountId.hashCode) +
      (reason == null ? 0 : reason.hashCode) +
      (governanceEventId == null ? 0 : governanceEventId.hashCode) +
      createdAt.hashCode;

  factory TrustLevelEvent.fromJson(Map<String, dynamic> json) =>
      _$TrustLevelEventFromJson(json);

  Map<String, dynamic> toJson() => _$TrustLevelEventToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum TrustLevelEventEventKindEnum {
  @JsonValue(r'upgrade')
  upgrade(r'upgrade'),
  @JsonValue(r'demotion')
  demotion(r'demotion'),
  @JsonValue(r'manual_set')
  manualSet(r'manual_set'),
  @JsonValue(r'override_clear')
  overrideClear(r'override_clear'),
  @JsonValue(r'backfill_initialized')
  backfillInitialized(r'backfill_initialized'),
  @JsonValue(r'registration')
  registration(r'registration'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const TrustLevelEventEventKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum TrustLevelEventActorKindEnum {
  @JsonValue(r'system')
  system(r'system'),
  @JsonValue(r'account')
  account(r'account'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const TrustLevelEventActorKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
