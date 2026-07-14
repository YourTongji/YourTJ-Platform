//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'mod_action.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ModAction {
  /// Returns a new [ModAction] instance.
  ModAction({
    this.id,

    this.actorId,

    this.action,

    this.targetType,

    this.targetId,

    this.reason,

    this.createdAt,
  });

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'actorId', required: false, includeIfNull: false)
  final String? actorId;

  @JsonKey(name: r'action', required: false, includeIfNull: false)
  final String? action;

  @JsonKey(name: r'targetType', required: false, includeIfNull: false)
  final String? targetType;

  @JsonKey(name: r'targetId', required: false, includeIfNull: false)
  final String? targetId;

  @JsonKey(name: r'reason', required: false, includeIfNull: false)
  final String? reason;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ModAction &&
          other.id == id &&
          other.actorId == actorId &&
          other.action == action &&
          other.targetType == targetType &&
          other.targetId == targetId &&
          other.reason == reason &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      actorId.hashCode +
      action.hashCode +
      targetType.hashCode +
      targetId.hashCode +
      (reason == null ? 0 : reason.hashCode) +
      createdAt.hashCode;

  factory ModAction.fromJson(Map<String, dynamic> json) =>
      _$ModActionFromJson(json);

  Map<String, dynamic> toJson() => _$ModActionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
