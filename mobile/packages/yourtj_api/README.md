# yourtj_api

Generated Dart/Dio bindings for `../../../contract/openapi.yaml`. Files under `lib/` are replaced by
`../../../scripts/generate_mobile_api.sh` and must not be edited by hand.

From the repository root, with Java and Dart available:

```bash
./scripts/generate_mobile_api.sh
```

The script pins and verifies OpenAPI Generator before generating serializers, formatting, applying safe
unused-import fixes, and running the package analyzer and generated-client contract tests.

OpenAPI Generator 7.22.0's `dart-dio` + `json_serializable` target flattens a discriminated `oneOf`
into a model that requires fields from every member. The script therefore overlays the single audited
`model_overrides/forum_draft_payload.dart` decoder before running `build_runner`. The OpenAPI
`ForumDraftPayload` union and its `kind` values remain the wire-contract source of truth; do not add
another override without a generated-client contract test and an explicit generator limitation.
