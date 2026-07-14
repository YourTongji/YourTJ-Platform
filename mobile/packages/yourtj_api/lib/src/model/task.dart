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
    this.id,

    this.creatorId,

    this.acceptorId,

    this.title,

    this.description,

    this.rewardAmount,

    this.contactInfo,

    this.status,

    this.createdAt,
  });

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'creatorId', required: false, includeIfNull: false)
  final String? creatorId;

  @JsonKey(name: r'acceptorId', required: false, includeIfNull: false)
  final String? acceptorId;

  @JsonKey(name: r'title', required: false, includeIfNull: false)
  final String? title;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  @JsonKey(name: r'rewardAmount', required: false, includeIfNull: false)
  final int? rewardAmount;

  /// Visible only to controlled parties
  @JsonKey(name: r'contactInfo', required: false, includeIfNull: false)
  final String? contactInfo;

  @JsonKey(
    name: r'status',
    required: false,
    includeIfNull: false,
    unknownEnumValue: TaskStatusEnum.unknownDefaultOpenApi,
  )
  final TaskStatusEnum? status;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

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
