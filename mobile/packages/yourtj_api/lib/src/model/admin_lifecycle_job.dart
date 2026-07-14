//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/account_lifecycle_state.dart';
import 'package:json_annotation/json_annotation.dart';

part 'admin_lifecycle_job.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminLifecycleJob {
  /// Returns a new [AdminLifecycleJob] instance.
  AdminLifecycleJob({
    required this.id,

    required this.accountId,

    required this.accountHandle,

    required this.accountState,

    required this.jobType,

    required this.status,

    required this.attempts,

    required this.nextAttemptAt,

    this.lockedAt,

    this.lastErrorCode,

    this.purgeStartedAt,

    required this.createdAt,

    required this.updatedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(name: r'accountHandle', required: true, includeIfNull: false)
  final String accountHandle;

  @JsonKey(
    name: r'accountState',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AccountLifecycleState.unknownDefaultOpenApi,
  )
  final AccountLifecycleState accountState;

  @JsonKey(
    name: r'jobType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminLifecycleJobJobTypeEnum.unknownDefaultOpenApi,
  )
  final AdminLifecycleJobJobTypeEnum jobType;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminLifecycleJobStatusEnum.unknownDefaultOpenApi,
  )
  final AdminLifecycleJobStatusEnum status;

  // minimum: 0
  // maximum: 20
  @JsonKey(name: r'attempts', required: true, includeIfNull: false)
  final int attempts;

  @JsonKey(name: r'nextAttemptAt', required: true, includeIfNull: false)
  final int nextAttemptAt;

  @JsonKey(name: r'lockedAt', required: false, includeIfNull: false)
  final int? lockedAt;

  @JsonKey(name: r'lastErrorCode', required: false, includeIfNull: false)
  final String? lastErrorCode;

  @JsonKey(name: r'purgeStartedAt', required: false, includeIfNull: false)
  final int? purgeStartedAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminLifecycleJob &&
          other.id == id &&
          other.accountId == accountId &&
          other.accountHandle == accountHandle &&
          other.accountState == accountState &&
          other.jobType == jobType &&
          other.status == status &&
          other.attempts == attempts &&
          other.nextAttemptAt == nextAttemptAt &&
          other.lockedAt == lockedAt &&
          other.lastErrorCode == lastErrorCode &&
          other.purgeStartedAt == purgeStartedAt &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      id.hashCode +
      accountId.hashCode +
      accountHandle.hashCode +
      accountState.hashCode +
      jobType.hashCode +
      status.hashCode +
      attempts.hashCode +
      nextAttemptAt.hashCode +
      (lockedAt == null ? 0 : lockedAt.hashCode) +
      (lastErrorCode == null ? 0 : lastErrorCode.hashCode) +
      (purgeStartedAt == null ? 0 : purgeStartedAt.hashCode) +
      createdAt.hashCode +
      updatedAt.hashCode;

  factory AdminLifecycleJob.fromJson(Map<String, dynamic> json) =>
      _$AdminLifecycleJobFromJson(json);

  Map<String, dynamic> toJson() => _$AdminLifecycleJobToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AdminLifecycleJobJobTypeEnum {
  @JsonValue(r'mark_deleted')
  markDeleted(r'mark_deleted'),
  @JsonValue(r'purge')
  purge(r'purge'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminLifecycleJobJobTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AdminLifecycleJobStatusEnum {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'running')
  running(r'running'),
  @JsonValue(r'succeeded')
  succeeded(r'succeeded'),
  @JsonValue(r'failed')
  failed(r'failed'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminLifecycleJobStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
