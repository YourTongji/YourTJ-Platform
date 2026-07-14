//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'in_app_notification_prefs.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class InAppNotificationPrefs {
  /// Returns a new [InAppNotificationPrefs] instance.
  InAppNotificationPrefs({
    this.replies = true,

    this.mentions = true,

    this.quotes = true,

    this.votes = true,

    this.badges = true,

    this.follows = true,

    this.subscriptions = true,

    this.directMessages = true,
  });

  @JsonKey(
    defaultValue: true,
    name: r'replies',
    required: true,
    includeIfNull: false,
  )
  final bool replies;

  @JsonKey(
    defaultValue: true,
    name: r'mentions',
    required: true,
    includeIfNull: false,
  )
  final bool mentions;

  @JsonKey(
    defaultValue: true,
    name: r'quotes',
    required: true,
    includeIfNull: false,
  )
  final bool quotes;

  @JsonKey(
    defaultValue: true,
    name: r'votes',
    required: true,
    includeIfNull: false,
  )
  final bool votes;

  @JsonKey(
    defaultValue: true,
    name: r'badges',
    required: true,
    includeIfNull: false,
  )
  final bool badges;

  @JsonKey(
    defaultValue: true,
    name: r'follows',
    required: true,
    includeIfNull: false,
  )
  final bool follows;

  @JsonKey(
    defaultValue: true,
    name: r'subscriptions',
    required: true,
    includeIfNull: false,
  )
  final bool subscriptions;

  @JsonKey(
    defaultValue: true,
    name: r'directMessages',
    required: true,
    includeIfNull: false,
  )
  final bool directMessages;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is InAppNotificationPrefs &&
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

  factory InAppNotificationPrefs.fromJson(Map<String, dynamic> json) =>
      _$InAppNotificationPrefsFromJson(json);

  Map<String, dynamic> toJson() => _$InAppNotificationPrefsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
