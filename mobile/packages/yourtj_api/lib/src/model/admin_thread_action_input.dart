//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_thread_action_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminThreadActionInput {
  /// Returns a new [AdminThreadActionInput] instance.
  AdminThreadActionInput({required this.reason, this.globally, this.boardId});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'globally', required: false, includeIfNull: false)
  final bool? globally;

  @JsonKey(name: r'boardId', required: false, includeIfNull: false)
  final String? boardId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminThreadActionInput &&
          other.reason == reason &&
          other.globally == globally &&
          other.boardId == boardId;

  @override
  int get hashCode => reason.hashCode + globally.hashCode + boardId.hashCode;

  factory AdminThreadActionInput.fromJson(Map<String, dynamic> json) =>
      _$AdminThreadActionInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminThreadActionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
