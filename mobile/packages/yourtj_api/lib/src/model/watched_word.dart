//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'watched_word.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class WatchedWord {
  /// Returns a new [WatchedWord] instance.
  WatchedWord({this.id, this.word, this.action, this.createdAt});

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'word', required: false, includeIfNull: false)
  final String? word;

  @JsonKey(
    name: r'action',
    required: false,
    includeIfNull: false,
    unknownEnumValue: WatchedWordActionEnum.unknownDefaultOpenApi,
  )
  final WatchedWordActionEnum? action;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WatchedWord &&
          other.id == id &&
          other.word == word &&
          other.action == action &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode + word.hashCode + action.hashCode + createdAt.hashCode;

  factory WatchedWord.fromJson(Map<String, dynamic> json) =>
      _$WatchedWordFromJson(json);

  Map<String, dynamic> toJson() => _$WatchedWordToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum WatchedWordActionEnum {
  @JsonValue(r'block')
  block(r'block'),
  @JsonValue(r'censor')
  censor(r'censor'),
  @JsonValue(r'queue')
  queue(r'queue'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const WatchedWordActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
