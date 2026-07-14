//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_audit_event.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminAuditEvent {
  /// Returns a new [AdminAuditEvent] instance.
  AdminAuditEvent({
    required this.id,

    required this.actorKind,

    this.actorId,

    this.actorHandle,

    this.actorRole,

    required this.action,

    required this.targetType,

    required this.targetId,

    this.reason,

    this.metadata,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'actorKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminAuditEventActorKindEnum.unknownDefaultOpenApi,
  )
  final AdminAuditEventActorKindEnum actorKind;

  @JsonKey(name: r'actorId', required: false, includeIfNull: false)
  final String? actorId;

  @JsonKey(name: r'actorHandle', required: false, includeIfNull: false)
  final String? actorHandle;

  @JsonKey(name: r'actorRole', required: false, includeIfNull: false)
  final String? actorRole;

  @JsonKey(name: r'action', required: true, includeIfNull: false)
  final String action;

  @JsonKey(name: r'targetType', required: true, includeIfNull: false)
  final String targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @JsonKey(name: r'reason', required: false, includeIfNull: false)
  final String? reason;

  @JsonKey(name: r'metadata', required: false, includeIfNull: false)
  final Object? metadata;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminAuditEvent &&
          other.id == id &&
          other.actorKind == actorKind &&
          other.actorId == actorId &&
          other.actorHandle == actorHandle &&
          other.actorRole == actorRole &&
          other.action == action &&
          other.targetType == targetType &&
          other.targetId == targetId &&
          other.reason == reason &&
          other.metadata == metadata &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      actorKind.hashCode +
      (actorId == null ? 0 : actorId.hashCode) +
      (actorHandle == null ? 0 : actorHandle.hashCode) +
      (actorRole == null ? 0 : actorRole.hashCode) +
      action.hashCode +
      targetType.hashCode +
      targetId.hashCode +
      (reason == null ? 0 : reason.hashCode) +
      (metadata == null ? 0 : metadata.hashCode) +
      createdAt.hashCode;

  factory AdminAuditEvent.fromJson(Map<String, dynamic> json) =>
      _$AdminAuditEventFromJson(json);

  Map<String, dynamic> toJson() => _$AdminAuditEventToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AdminAuditEventActorKindEnum {
  @JsonValue(r'account')
  account(r'account'),
  @JsonValue(r'system')
  system(r'system'),
  @JsonValue(r'service')
  service(r'service'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminAuditEventActorKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
