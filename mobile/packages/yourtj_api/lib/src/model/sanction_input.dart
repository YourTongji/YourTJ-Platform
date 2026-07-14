//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'sanction_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SanctionInput {
  /// Returns a new [SanctionInput] instance.
  SanctionInput({required this.reason, this.endsAt});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SanctionInput &&
          other.reason == reason &&
          other.endsAt == endsAt;

  @override
  int get hashCode => reason.hashCode + (endsAt == null ? 0 : endsAt.hashCode);

  factory SanctionInput.fromJson(Map<String, dynamic> json) =>
      _$SanctionInputFromJson(json);

  Map<String, dynamic> toJson() => _$SanctionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
