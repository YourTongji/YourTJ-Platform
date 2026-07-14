//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_read_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmReadInput {
  /// Returns a new [DmReadInput] instance.
  DmReadInput({this.lastReadMessageId});

  @JsonKey(name: r'lastReadMessageId', required: false, includeIfNull: false)
  final String? lastReadMessageId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmReadInput && other.lastReadMessageId == lastReadMessageId;

  @override
  int get hashCode =>
      (lastReadMessageId == null ? 0 : lastReadMessageId.hashCode);

  factory DmReadInput.fromJson(Map<String, dynamic> json) =>
      _$DmReadInputFromJson(json);

  Map<String, dynamic> toJson() => _$DmReadInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
