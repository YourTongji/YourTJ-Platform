//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'media_retention_hold_release_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaRetentionHoldReleaseInput {
  /// Returns a new [MediaRetentionHoldReleaseInput] instance.
  MediaRetentionHoldReleaseInput({
    required this.expectedHoldId,

    required this.reason,
  });

  /// The exact reviewed hold to release.
  @JsonKey(name: r'expectedHoldId', required: true, includeIfNull: false)
  final String expectedHoldId;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaRetentionHoldReleaseInput &&
          other.expectedHoldId == expectedHoldId &&
          other.reason == reason;

  @override
  int get hashCode => expectedHoldId.hashCode + reason.hashCode;

  factory MediaRetentionHoldReleaseInput.fromJson(Map<String, dynamic> json) =>
      _$MediaRetentionHoldReleaseInputFromJson(json);

  Map<String, dynamic> toJson() => _$MediaRetentionHoldReleaseInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
