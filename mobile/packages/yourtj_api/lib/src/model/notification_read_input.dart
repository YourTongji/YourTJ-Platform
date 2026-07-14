//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'notification_read_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationReadInput {
  /// Returns a new [NotificationReadInput] instance.
  NotificationReadInput({this.ids, this.all});

  /// Mark only these notifications. They must belong to the current account.
  @JsonKey(name: r'ids', required: false, includeIfNull: false)
  final List<String>? ids;

  /// Mark every notification for the current account. Cannot be combined with ids.
  @JsonKey(name: r'all', required: false, includeIfNull: false)
  final bool? all;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationReadInput && other.ids == ids && other.all == all;

  @override
  int get hashCode => ids.hashCode + all.hashCode;

  factory NotificationReadInput.fromJson(Map<String, dynamic> json) =>
      _$NotificationReadInputFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationReadInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
