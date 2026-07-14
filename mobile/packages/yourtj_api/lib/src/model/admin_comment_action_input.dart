//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_comment_action_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminCommentActionInput {
  /// Returns a new [AdminCommentActionInput] instance.
  AdminCommentActionInput({required this.reason});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminCommentActionInput && other.reason == reason;

  @override
  int get hashCode => reason.hashCode;

  factory AdminCommentActionInput.fromJson(Map<String, dynamic> json) =>
      _$AdminCommentActionInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminCommentActionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
