//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'task.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Task {
  /// Returns a new [Task] instance.
  Task({
    required this.id,

    required this.creatorId,

    required this.acceptorId,

    required this.title,

    required this.description,

    required this.rewardAmount,

    required this.contactInfo,

    required this.status,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'creatorId', required: true, includeIfNull: false)
  final String creatorId;

  @JsonKey(name: r'acceptorId', required: true, includeIfNull: true)
  final String? acceptorId;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'description', required: true, includeIfNull: true)
  final String? description;

  @JsonKey(name: r'rewardAmount', required: true, includeIfNull: false)
  final int rewardAmount;

  /// Visible only to controlled parties
  @JsonKey(name: r'contactInfo', required: true, includeIfNull: true)
  final String? contactInfo;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: TaskStatusEnum.unknownDefaultOpenApi,
  )
  final TaskStatusEnum status;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Task &&
          other.id == id &&
          other.creatorId == creatorId &&
          other.acceptorId == acceptorId &&
          other.title == title &&
          other.description == description &&
          other.rewardAmount == rewardAmount &&
          other.contactInfo == contactInfo &&
          other.status == status &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      creatorId.hashCode +
      (acceptorId == null ? 0 : acceptorId.hashCode) +
      title.hashCode +
      (description == null ? 0 : description.hashCode) +
      rewardAmount.hashCode +
      (contactInfo == null ? 0 : contactInfo.hashCode) +
      status.hashCode +
      createdAt.hashCode;

  factory Task.fromJson(Map<String, dynamic> json) => _$TaskFromJson(json);

  Map<String, dynamic> toJson() => _$TaskToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum TaskStatusEnum {
  @JsonValue(r'open')
  open(r'open'),
  @JsonValue(r'in_progress')
  inProgress(r'in_progress'),
  @JsonValue(r'submitted')
  submitted(r'submitted'),
  @JsonValue(r'completed')
  completed(r'completed'),
  @JsonValue(r'cancelled')
  cancelled(r'cancelled'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const TaskStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
