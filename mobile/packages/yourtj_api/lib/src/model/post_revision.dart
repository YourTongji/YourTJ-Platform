//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:json_annotation/json_annotation.dart';

part 'post_revision.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PostRevision {
  /// Returns a new [PostRevision] instance.
  PostRevision({
    required this.id,

    required this.seq,

    required this.editorId,

    required this.oldTitle,

    required this.oldBody,

    required this.oldContentFormat,

    required this.oldContentVersion,

    required this.attachments,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'seq', required: true, includeIfNull: false)
  final int seq;

  @JsonKey(name: r'editorId', required: true, includeIfNull: false)
  final String editorId;

  @JsonKey(name: r'oldTitle', required: true, includeIfNull: true)
  final String? oldTitle;

  @JsonKey(name: r'oldBody', required: true, includeIfNull: false)
  final String oldBody;

  @JsonKey(
    name: r'oldContentFormat',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat oldContentFormat;

  // minimum: 1
  @JsonKey(name: r'oldContentVersion', required: true, includeIfNull: false)
  final int oldContentVersion;

  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PostRevision &&
          other.id == id &&
          other.seq == seq &&
          other.editorId == editorId &&
          other.oldTitle == oldTitle &&
          other.oldBody == oldBody &&
          other.oldContentFormat == oldContentFormat &&
          other.oldContentVersion == oldContentVersion &&
          other.attachments == attachments &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      seq.hashCode +
      editorId.hashCode +
      (oldTitle == null ? 0 : oldTitle.hashCode) +
      oldBody.hashCode +
      oldContentFormat.hashCode +
      oldContentVersion.hashCode +
      attachments.hashCode +
      createdAt.hashCode;

  factory PostRevision.fromJson(Map<String, dynamic> json) =>
      _$PostRevisionFromJson(json);

  Map<String, dynamic> toJson() => _$PostRevisionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
