//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'watched_word_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class WatchedWordInput {
  /// Returns a new [WatchedWordInput] instance.
  WatchedWordInput({
    required this.word,

    required this.action,

    required this.reason,
  });

  @JsonKey(name: r'word', required: true, includeIfNull: false)
  final String word;

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: WatchedWordInputActionEnum.unknownDefaultOpenApi,
  )
  final WatchedWordInputActionEnum action;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WatchedWordInput &&
          other.word == word &&
          other.action == action &&
          other.reason == reason;

  @override
  int get hashCode => word.hashCode + action.hashCode + reason.hashCode;

  factory WatchedWordInput.fromJson(Map<String, dynamic> json) =>
      _$WatchedWordInputFromJson(json);

  Map<String, dynamic> toJson() => _$WatchedWordInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum WatchedWordInputActionEnum {
  @JsonValue(r'block')
  block(r'block'),
  @JsonValue(r'censor')
  censor(r'censor'),
  @JsonValue(r'queue')
  queue(r'queue'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const WatchedWordInputActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
