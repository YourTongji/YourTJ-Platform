//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'thread_move_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadMoveInput {
  /// Returns a new [ThreadMoveInput] instance.
  ThreadMoveInput({required this.boardId});

  @JsonKey(name: r'boardId', required: true, includeIfNull: false)
  final String boardId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadMoveInput && other.boardId == boardId;

  @override
  int get hashCode => boardId.hashCode;

  factory ThreadMoveInput.fromJson(Map<String, dynamic> json) =>
      _$ThreadMoveInputFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadMoveInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
