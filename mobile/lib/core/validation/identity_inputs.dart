abstract final class IdentityInputs {
  static final RegExp _publicHandle = RegExp(r'^[a-z0-9._-]{3,30}$');
  static final RegExp _emailVerificationCode = RegExp(r'^[0-9]{6}$');

  static bool isValidPublicHandle(String value) =>
      _publicHandle.hasMatch(value);

  static bool isValidEmailVerificationCode(String value) =>
      _emailVerificationCode.hasMatch(value);
}
