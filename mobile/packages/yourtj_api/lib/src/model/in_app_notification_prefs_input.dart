//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'in_app_notification_prefs_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class InAppNotificationPrefsInput {
  /// Returns a new [InAppNotificationPrefsInput] instance.
  InAppNotificationPrefsInput({
    required this.replies,

    required this.mentions,

    required this.quotes,

    required this.votes,

    required this.badges,

    this.follows,

    required this.subscriptions,

    required this.directMessages,
  });

  @JsonKey(name: r'replies', required: true, includeIfNull: false)
  final bool replies;

  @JsonKey(name: r'mentions', required: true, includeIfNull: false)
  final bool mentions;

  @JsonKey(name: r'quotes', required: true, includeIfNull: false)
  final bool quotes;

  @JsonKey(name: r'votes', required: true, includeIfNull: false)
  final bool votes;

  @JsonKey(name: r'badges', required: true, includeIfNull: false)
  final bool badges;

  /// Optional only during rolling deployment; omission preserves the stored value.
  @JsonKey(name: r'follows', required: false, includeIfNull: false)
  final bool? follows;

  @JsonKey(name: r'subscriptions', required: true, includeIfNull: false)
  final bool subscriptions;

  @JsonKey(name: r'directMessages', required: true, includeIfNull: false)
  final bool directMessages;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is InAppNotificationPrefsInput &&
          other.replies == replies &&
          other.mentions == mentions &&
          other.quotes == quotes &&
          other.votes == votes &&
          other.badges == badges &&
          other.follows == follows &&
          other.subscriptions == subscriptions &&
          other.directMessages == directMessages;

  @override
  int get hashCode =>
      replies.hashCode +
      mentions.hashCode +
      quotes.hashCode +
      votes.hashCode +
      badges.hashCode +
      follows.hashCode +
      subscriptions.hashCode +
      directMessages.hashCode;

  factory InAppNotificationPrefsInput.fromJson(Map<String, dynamic> json) =>
      _$InAppNotificationPrefsInputFromJson(json);

  Map<String, dynamic> toJson() => _$InAppNotificationPrefsInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
