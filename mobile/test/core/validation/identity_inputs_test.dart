import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/validation/identity_inputs.dart';

void main() {
  test('public handle matches the OpenAPI alphabet and bounds', () {
    expect(IdentityInputs.isValidPublicHandle('abc'), isTrue);
    expect(
      IdentityInputs.isValidPublicHandle('student.name-test_2026'),
      isTrue,
    );
    expect(IdentityInputs.isValidPublicHandle('a' * 30), isTrue);

    expect(IdentityInputs.isValidPublicHandle('ab'), isFalse);
    expect(IdentityInputs.isValidPublicHandle('a' * 31), isFalse);
    expect(IdentityInputs.isValidPublicHandle('Student'), isFalse);
    expect(IdentityInputs.isValidPublicHandle('student name'), isFalse);
  });

  test('email verification code is exactly six ASCII digits', () {
    expect(IdentityInputs.isValidEmailVerificationCode('012345'), isTrue);

    expect(IdentityInputs.isValidEmailVerificationCode('12345'), isFalse);
    expect(IdentityInputs.isValidEmailVerificationCode('1234567'), isFalse);
    expect(IdentityInputs.isValidEmailVerificationCode('12345a'), isFalse);
    expect(IdentityInputs.isValidEmailVerificationCode(' 123456'), isFalse);
  });
}
