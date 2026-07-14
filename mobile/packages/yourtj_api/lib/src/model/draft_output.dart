//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/forum_draft_payload.dart';
import 'package:json_annotation/json_annotation.dart';

part 'draft_output.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DraftOutput {
  /// Returns a new [DraftOutput] instance.
  DraftOutput({
    required this.draftKey,

    required this.payload,

    required this.version,

    required this.updatedAt,
  });

  @JsonKey(name: r'draftKey', required: true, includeIfNull: false)
  final String draftKey;

  @JsonKey(name: r'payload', required: true, includeIfNull: false)
  final ForumDraftPayload payload;

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DraftOutput &&
          other.draftKey == draftKey &&
          other.payload == payload &&
          other.version == version &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      draftKey.hashCode +
      payload.hashCode +
      version.hashCode +
      updatedAt.hashCode;

  factory DraftOutput.fromJson(Map<String, dynamic> json) =>
      _$DraftOutputFromJson(json);

  Map<String, dynamic> toJson() => _$DraftOutputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
