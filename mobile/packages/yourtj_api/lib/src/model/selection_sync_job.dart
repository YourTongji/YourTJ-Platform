//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'selection_sync_job.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SelectionSyncJob {
  /// Returns a new [SelectionSyncJob] instance.
  SelectionSyncJob({
    required this.id,

    required this.requestedBy,

    required this.status,

    required this.step,

    required this.attempts,

    required this.progressCurrent,

    required this.progressTotal,

    required this.nextAttemptAt,

    required this.lastErrorCode,

    required this.result,

    this.startedAt,

    this.completedAt,

    required this.createdAt,

    required this.updatedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'requestedBy', required: true, includeIfNull: false)
  final String requestedBy;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SelectionSyncJobStatusEnum.unknownDefaultOpenApi,
  )
  final SelectionSyncJobStatusEnum status;

  @JsonKey(
    name: r'step',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SelectionSyncJobStepEnum.unknownDefaultOpenApi,
  )
  final SelectionSyncJobStepEnum step;

  // minimum: 0
  // maximum: 8
  @JsonKey(name: r'attempts', required: true, includeIfNull: false)
  final int attempts;

  // minimum: 0
  // maximum: 4
  @JsonKey(name: r'progressCurrent', required: true, includeIfNull: false)
  final int progressCurrent;

  @JsonKey(
    name: r'progressTotal',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SelectionSyncJobProgressTotalEnum.unknownDefaultOpenApi,
  )
  final SelectionSyncJobProgressTotalEnum progressTotal;

  @JsonKey(name: r'nextAttemptAt', required: true, includeIfNull: false)
  final int nextAttemptAt;

  @JsonKey(name: r'lastErrorCode', required: true, includeIfNull: true)
  final String? lastErrorCode;

  /// Bounded aggregate counts only; no raw timetable records.
  @JsonKey(name: r'result', required: true, includeIfNull: false)
  final Map<String, Object> result;

  @JsonKey(name: r'startedAt', required: false, includeIfNull: false)
  final int? startedAt;

  @JsonKey(name: r'completedAt', required: false, includeIfNull: false)
  final int? completedAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SelectionSyncJob &&
          other.id == id &&
          other.requestedBy == requestedBy &&
          other.status == status &&
          other.step == step &&
          other.attempts == attempts &&
          other.progressCurrent == progressCurrent &&
          other.progressTotal == progressTotal &&
          other.nextAttemptAt == nextAttemptAt &&
          other.lastErrorCode == lastErrorCode &&
          other.result == result &&
          other.startedAt == startedAt &&
          other.completedAt == completedAt &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      id.hashCode +
      requestedBy.hashCode +
      status.hashCode +
      step.hashCode +
      attempts.hashCode +
      progressCurrent.hashCode +
      progressTotal.hashCode +
      nextAttemptAt.hashCode +
      (lastErrorCode == null ? 0 : lastErrorCode.hashCode) +
      result.hashCode +
      (startedAt == null ? 0 : startedAt.hashCode) +
      (completedAt == null ? 0 : completedAt.hashCode) +
      createdAt.hashCode +
      updatedAt.hashCode;

  factory SelectionSyncJob.fromJson(Map<String, dynamic> json) =>
      _$SelectionSyncJobFromJson(json);

  Map<String, dynamic> toJson() => _$SelectionSyncJobToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum SelectionSyncJobStatusEnum {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'running')
  running(r'running'),
  @JsonValue(r'succeeded')
  succeeded(r'succeeded'),
  @JsonValue(r'dead')
  dead(r'dead'),
  @JsonValue(r'cancelled')
  cancelled(r'cancelled'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SelectionSyncJobStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum SelectionSyncJobStepEnum {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'materialize')
  materialize(r'materialize'),
  @JsonValue(r'catalogue')
  catalogue(r'catalogue'),
  @JsonValue(r'search')
  search(r'search'),
  @JsonValue(r'cache')
  cache(r'cache'),
  @JsonValue(r'complete')
  complete(r'complete'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SelectionSyncJobStepEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum SelectionSyncJobProgressTotalEnum {
  @JsonValue(4)
  number4('4'),
  @JsonValue(11184809)
  unknownDefaultOpenApi('11184809');

  const SelectionSyncJobProgressTotalEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
