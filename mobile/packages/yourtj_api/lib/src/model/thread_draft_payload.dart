//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:json_annotation/json_annotation.dart';

part 'thread_draft_payload.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadDraftPayload {
  /// Returns a new [ThreadDraftPayload] instance.
  ThreadDraftPayload({
    required this.kind,

    required this.boardId,

    required this.title,

    required this.body,

    required this.contentFormat,

    required this.tags,

    required this.pollQuestion,

    required this.pollOptions,

    required this.attachmentAssetIds,
  });

  @JsonKey(
    name: r'kind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ThreadDraftPayloadKindEnum.unknownDefaultOpenApi,
  )
  final ThreadDraftPayloadKindEnum kind;

  @JsonKey(name: r'boardId', required: true, includeIfNull: true)
  final String? boardId;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @JsonKey(
    name: r'contentFormat',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat contentFormat;

  @JsonKey(name: r'tags', required: true, includeIfNull: false)
  final List<String> tags;

  @JsonKey(name: r'pollQuestion', required: true, includeIfNull: false)
  final String pollQuestion;

  @JsonKey(name: r'pollOptions', required: true, includeIfNull: false)
  final List<String> pollOptions;

  /// Owner-only upload ids retained across devices. Pending/blocked state is not public binding authorization.
  @JsonKey(name: r'attachmentAssetIds', required: true, includeIfNull: false)
  final Set<String> attachmentAssetIds;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadDraftPayload &&
          other.kind == kind &&
          other.boardId == boardId &&
          other.title == title &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.tags == tags &&
          other.pollQuestion == pollQuestion &&
          other.pollOptions == pollOptions &&
          other.attachmentAssetIds == attachmentAssetIds;

  @override
  int get hashCode =>
      kind.hashCode +
      (boardId == null ? 0 : boardId.hashCode) +
      title.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      tags.hashCode +
      pollQuestion.hashCode +
      pollOptions.hashCode +
      attachmentAssetIds.hashCode;

  factory ThreadDraftPayload.fromJson(Map<String, dynamic> json) =>
      _$ThreadDraftPayloadFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadDraftPayloadToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ThreadDraftPayloadKindEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ThreadDraftPayloadKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
