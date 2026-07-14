//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'bookmark_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class BookmarkInput {
  /// Returns a new [BookmarkInput] instance.
  BookmarkInput({required this.postType, this.note});

  @JsonKey(
    name: r'postType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: BookmarkInputPostTypeEnum.unknownDefaultOpenApi,
  )
  final BookmarkInputPostTypeEnum postType;

  @JsonKey(name: r'note', required: false, includeIfNull: false)
  final String? note;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is BookmarkInput &&
          other.postType == postType &&
          other.note == note;

  @override
  int get hashCode => postType.hashCode + note.hashCode;

  factory BookmarkInput.fromJson(Map<String, dynamic> json) =>
      _$BookmarkInputFromJson(json);

  Map<String, dynamic> toJson() => _$BookmarkInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum BookmarkInputPostTypeEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const BookmarkInputPostTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
