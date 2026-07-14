//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:json_annotation/json_annotation.dart';

part 'thread_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadUpdateInput {
  /// Returns a new [ThreadUpdateInput] instance.
  ThreadUpdateInput({
    this.expectedVersion = 1,

    this.title,

    this.body,

    this.contentFormat,

    this.tags,

    this.attachmentAssetIds,
  });

  /// Compare-and-swap version. Legacy omission is treated as version 1 and conflicts once content has changed.
  // minimum: 1
  @JsonKey(
    defaultValue: 1,
    name: r'expectedVersion',
    required: false,
    includeIfNull: false,
  )
  final int? expectedVersion;

  @JsonKey(name: r'title', required: false, includeIfNull: false)
  final String? title;

  @JsonKey(name: r'body', required: false, includeIfNull: false)
  final String? body;

  /// Required whenever body is supplied; legacy omission is treated as plain_v1.
  @JsonKey(
    name: r'contentFormat',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat? contentFormat;

  @JsonKey(name: r'tags', required: false, includeIfNull: false)
  final Set<String>? tags;

  /// When body is supplied, ordered ids must exactly match every yourtj-asset image reference; otherwise omit or send an empty array.
  @JsonKey(name: r'attachmentAssetIds', required: false, includeIfNull: false)
  final Set<String>? attachmentAssetIds;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadUpdateInput &&
          other.expectedVersion == expectedVersion &&
          other.title == title &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.tags == tags &&
          other.attachmentAssetIds == attachmentAssetIds;

  @override
  int get hashCode =>
      expectedVersion.hashCode +
      title.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      tags.hashCode +
      attachmentAssetIds.hashCode;

  factory ThreadUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$ThreadUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
