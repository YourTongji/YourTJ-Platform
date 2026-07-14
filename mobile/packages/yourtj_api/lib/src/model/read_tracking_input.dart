//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'read_tracking_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ReadTrackingInput {
  /// Returns a new [ReadTrackingInput] instance.
  ReadTrackingInput({this.lastReadCommentId});

  /// When null or omitted, mark through the thread's current last visible comment.
  @JsonKey(name: r'lastReadCommentId', required: false, includeIfNull: false)
  final String? lastReadCommentId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ReadTrackingInput &&
          other.lastReadCommentId == lastReadCommentId;

  @override
  int get hashCode =>
      (lastReadCommentId == null ? 0 : lastReadCommentId.hashCode);

  factory ReadTrackingInput.fromJson(Map<String, dynamic> json) =>
      _$ReadTrackingInputFromJson(json);

  Map<String, dynamic> toJson() => _$ReadTrackingInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
