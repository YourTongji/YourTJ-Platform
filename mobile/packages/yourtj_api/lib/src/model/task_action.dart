//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'task_action.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TaskAction {
  /// Returns a new [TaskAction] instance.
  TaskAction({required this.action});

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: TaskActionActionEnum.unknownDefaultOpenApi,
  )
  final TaskActionActionEnum action;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || other is TaskAction && other.action == action;

  @override
  int get hashCode => action.hashCode;

  factory TaskAction.fromJson(Map<String, dynamic> json) =>
      _$TaskActionFromJson(json);

  Map<String, dynamic> toJson() => _$TaskActionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum TaskActionActionEnum {
  @JsonValue(r'submit')
  submit(r'submit'),
  @JsonValue(r'confirm')
  confirm(r'confirm'),
  @JsonValue(r'cancel')
  cancel(r'cancel'),
  @JsonValue(r'reject')
  reject(r'reject'),
  @JsonValue(r'delete')
  delete(r'delete'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const TaskActionActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
