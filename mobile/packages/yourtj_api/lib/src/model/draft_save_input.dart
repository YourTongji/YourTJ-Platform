//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/forum_draft_payload.dart';
import 'package:json_annotation/json_annotation.dart';

part 'draft_save_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DraftSaveInput {
  /// Returns a new [DraftSaveInput] instance.
  DraftSaveInput({
    required this.draftKey,

    required this.expectedVersion,

    required this.payload,
  });

  @JsonKey(name: r'draftKey', required: true, includeIfNull: false)
  final String draftKey;

  /// Zero creates a new draft; a positive value compare-and-swaps that version.
  // minimum: 0
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  @JsonKey(name: r'payload', required: true, includeIfNull: false)
  final ForumDraftPayload payload;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DraftSaveInput &&
          other.draftKey == draftKey &&
          other.expectedVersion == expectedVersion &&
          other.payload == payload;

  @override
  int get hashCode =>
      draftKey.hashCode + expectedVersion.hashCode + payload.hashCode;

  factory DraftSaveInput.fromJson(Map<String, dynamic> json) =>
      _$DraftSaveInputFromJson(json);

  Map<String, dynamic> toJson() => _$DraftSaveInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
