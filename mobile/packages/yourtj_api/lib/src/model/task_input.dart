//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'task_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TaskInput {
  /// Returns a new [TaskInput] instance.
  TaskInput({
    required this.title,

    this.description,

    required this.rewardAmount,

    this.contactInfo,
  });

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  // minimum: 1
  @JsonKey(name: r'rewardAmount', required: true, includeIfNull: false)
  final int rewardAmount;

  @JsonKey(name: r'contactInfo', required: false, includeIfNull: false)
  final String? contactInfo;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TaskInput &&
          other.title == title &&
          other.description == description &&
          other.rewardAmount == rewardAmount &&
          other.contactInfo == contactInfo;

  @override
  int get hashCode =>
      title.hashCode +
      description.hashCode +
      rewardAmount.hashCode +
      contactInfo.hashCode;

  factory TaskInput.fromJson(Map<String, dynamic> json) =>
      _$TaskInputFromJson(json);

  Map<String, dynamic> toJson() => _$TaskInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
