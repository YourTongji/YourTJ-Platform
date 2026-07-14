# YourTJ Mobile

Flutter Android/iOS client for YourTJ Community. Product behavior and parity requirements live in
`../docs/product/mobile-client.md`; HTTP types are owned by `../contract/openapi.yaml`.

The client now uses the generated Dart API for the adaptive five-destination shell, identity and
onboarding, home/feed/promotions, forum and media, courses/reviews/local schedule, federated search,
profiles/social actions, notifications/global announcements, direct messages, appeals/account lifecycle,
the signed wallet, and capability-gated administration. It also consumes the shared Markdown, Ed25519,
and OSS V4 conformance fixtures; feature pages do not substitute sample data for platform facts.

This breadth is still `Partial`, not a release claim. The canonical 19-row matrix records the remaining
contract/read-side gaps and the missing golden, real-environment integration, Android/iOS device,
hosted verified-link, release-signing, and store-rollout evidence. See
`../docs/product/mobile-client.md` before describing any journey as Web-parity complete.

## Local checks

Use Flutter 3.44.6 (Dart 3.12.2):

```bash
flutter pub get --enforce-lockfile
../scripts/generate_mobile_api.sh
dart format --output=none --set-exit-if-changed lib test
flutter analyze --fatal-infos --fatal-warnings
flutter test
flutter build apk --debug
flutter build apk --release
# macOS/Xcode only
flutter build ios --debug --no-codesign
flutter build ios --release --no-codesign
```

Run on an Android emulator or iOS Simulator with `flutter run`. The generated package lives in
`packages/yourtj_api`; generation is owned by the repository-root script above, not a `mobile/tool`
directory. Current CI compiles debug and unsigned release modes but does not publish them. App Store/Android release credentials,
hosted Digital Asset Links/AASA, signed-device verification, distribution, and rollback are not present
in this repository and must come from a controlled release environment.

Platform images are environment-bound. Production defaults to `https://media.yourtj.de`; every staging
or development build must pass the exact backend CDN origin with
`--dart-define=YOURTJ_MEDIA_CDN_BASE_URL=https://media-dev.example`. The client rejects third-party
HTTPS avatar URLs rather than treating TLS as proof that an origin is platform-owned.

## Source policy

The implementation is clean-room code. Do not copy source, assets, tests, visual constants, or local
packages from FluxDO or the legacy YourTJCourse Flutter client. Evaluate every added dependency from its
official source and record its license impact in the pull request.
