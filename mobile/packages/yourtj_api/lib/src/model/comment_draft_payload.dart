//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:json_annotation/json_annotation.dart';

part 'comment_draft_payload.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CommentDraftPayload {
  /// Returns a new [CommentDraftPayload] instance.
  CommentDraftPayload({
    required this.kind,

    required this.threadId,

    required this.body,

    required this.contentFormat,

    required this.parentId,

    required this.attachmentAssetIds,
  });

  @JsonKey(
    name: r'kind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: CommentDraftPayloadKindEnum.unknownDefaultOpenApi,
  )
  final CommentDraftPayloadKindEnum kind;

  @JsonKey(name: r'threadId', required: true, includeIfNull: false)
  final String threadId;

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @JsonKey(
    name: r'contentFormat',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat contentFormat;

  @JsonKey(name: r'parentId', required: true, includeIfNull: true)
  final String? parentId;

  /// Owner-only upload ids retained across devices. Pending/blocked state is not public binding authorization.
  @JsonKey(name: r'attachmentAssetIds', required: true, includeIfNull: false)
  final Set<String> attachmentAssetIds;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CommentDraftPayload &&
          other.kind == kind &&
          other.threadId == threadId &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.parentId == parentId &&
          other.attachmentAssetIds == attachmentAssetIds;

  @override
  int get hashCode =>
      kind.hashCode +
      threadId.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      (parentId == null ? 0 : parentId.hashCode) +
      attachmentAssetIds.hashCode;

  factory CommentDraftPayload.fromJson(Map<String, dynamic> json) =>
      _$CommentDraftPayloadFromJson(json);

  Map<String, dynamic> toJson() => _$CommentDraftPayloadToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum CommentDraftPayloadKindEnum {
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const CommentDraftPayloadKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
