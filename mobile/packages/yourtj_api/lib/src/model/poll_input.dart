//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'poll_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PollInput {
  /// Returns a new [PollInput] instance.
  PollInput({
    required this.question,

    this.multiSelect = false,

    this.closesAt,

    required this.options,
  });

  @JsonKey(name: r'question', required: true, includeIfNull: false)
  final String question;

  @JsonKey(
    defaultValue: false,
    name: r'multiSelect',
    required: false,
    includeIfNull: false,
  )
  final bool? multiSelect;

  @JsonKey(name: r'closesAt', required: false, includeIfNull: false)
  final int? closesAt;

  @JsonKey(name: r'options', required: true, includeIfNull: false)
  final Set<String> options;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PollInput &&
          other.question == question &&
          other.multiSelect == multiSelect &&
          other.closesAt == closesAt &&
          other.options == options;

  @override
  int get hashCode =>
      question.hashCode +
      multiSelect.hashCode +
      (closesAt == null ? 0 : closesAt.hashCode) +
      options.hashCode;

  factory PollInput.fromJson(Map<String, dynamic> json) =>
      _$PollInputFromJson(json);

  Map<String, dynamic> toJson() => _$PollInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
