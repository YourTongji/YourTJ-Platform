//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'media_moderation_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaModerationInput {
  /// Returns a new [MediaModerationInput] instance.
  MediaModerationInput({
    required this.reason,

    this.selfReviewConfirmed = false,
  });

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  /// Must be true only when ADMIN intentionally invokes the recent-authenticated own-media exception.
  @JsonKey(
    defaultValue: false,
    name: r'selfReviewConfirmed',
    required: false,
    includeIfNull: false,
  )
  final bool? selfReviewConfirmed;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaModerationInput &&
          other.reason == reason &&
          other.selfReviewConfirmed == selfReviewConfirmed;

  @override
  int get hashCode => reason.hashCode + selfReviewConfirmed.hashCode;

  factory MediaModerationInput.fromJson(Map<String, dynamic> json) =>
      _$MediaModerationInputFromJson(json);

  Map<String, dynamic> toJson() => _$MediaModerationInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
