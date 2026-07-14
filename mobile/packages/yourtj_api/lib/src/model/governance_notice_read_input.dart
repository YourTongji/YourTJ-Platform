//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'governance_notice_read_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class GovernanceNoticeReadInput {
  /// Returns a new [GovernanceNoticeReadInput] instance.
  GovernanceNoticeReadInput({this.ids, this.all});

  @JsonKey(name: r'ids', required: false, includeIfNull: false)
  final List<String>? ids;

  @JsonKey(name: r'all', required: false, includeIfNull: false)
  final bool? all;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is GovernanceNoticeReadInput &&
          other.ids == ids &&
          other.all == all;

  @override
  int get hashCode => ids.hashCode + all.hashCode;

  factory GovernanceNoticeReadInput.fromJson(Map<String, dynamic> json) =>
      _$GovernanceNoticeReadInputFromJson(json);

  Map<String, dynamic> toJson() => _$GovernanceNoticeReadInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
