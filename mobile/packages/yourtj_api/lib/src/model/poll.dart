//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/poll_option.dart';
import 'package:json_annotation/json_annotation.dart';

part 'poll.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Poll {
  /// Returns a new [Poll] instance.
  Poll({
    required this.id,

    required this.question,

    required this.multiSelect,

    required this.closesAt,

    required this.options,

    required this.myVotes,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'question', required: true, includeIfNull: false)
  final String question;

  @JsonKey(name: r'multiSelect', required: true, includeIfNull: false)
  final bool multiSelect;

  @JsonKey(name: r'closesAt', required: true, includeIfNull: true)
  final int? closesAt;

  @JsonKey(name: r'options', required: true, includeIfNull: false)
  final List<PollOption> options;

  @JsonKey(name: r'myVotes', required: true, includeIfNull: false)
  final List<String> myVotes;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Poll &&
          other.id == id &&
          other.question == question &&
          other.multiSelect == multiSelect &&
          other.closesAt == closesAt &&
          other.options == options &&
          other.myVotes == myVotes;

  @override
  int get hashCode =>
      id.hashCode +
      question.hashCode +
      multiSelect.hashCode +
      (closesAt == null ? 0 : closesAt.hashCode) +
      options.hashCode +
      myVotes.hashCode;

  factory Poll.fromJson(Map<String, dynamic> json) => _$PollFromJson(json);

  Map<String, dynamic> toJson() => _$PollToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
