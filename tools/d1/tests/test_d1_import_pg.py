from __future__ import annotations

import argparse
import importlib.util
import json
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "d1_import_pg.py"
MODULE_SPEC = importlib.util.spec_from_file_location("d1_import_pg_under_test", SCRIPT_PATH)
if MODULE_SPEC is None or MODULE_SPEC.loader is None:
    raise RuntimeError("could not load d1_import_pg.py")
D1_IMPORT = importlib.util.module_from_spec(MODULE_SPEC)
sys.modules[MODULE_SPEC.name] = D1_IMPORT
MODULE_SPEC.loader.exec_module(D1_IMPORT)


class ManifestSafetyTests(unittest.TestCase):
    @staticmethod
    def _counts(value: int = 1) -> dict[str, int]:
        return {spec.source: value for spec in D1_IMPORT.TABLES}

    @staticmethod
    def _approval_args(**overrides: object) -> argparse.Namespace:
        values: dict[str, object] = {
            "approve_unbaselined_snapshot": False,
            "approve_count_decrease": False,
            "approval_reason": None,
        }
        values.update(overrides)
        return argparse.Namespace(**values)

    def test_write_manifest_creates_private_file_without_overwrite(self) -> None:
        manifest = {"schemaVersion": 1, "sourceTableCounts": {"calendar": 1}}
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "manifest.json"
            D1_IMPORT.write_manifest(output, manifest)

            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), manifest)
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o600)

            original = output.read_bytes()
            with self.assertRaisesRegex(FileExistsError, "already exists"):
                D1_IMPORT.write_manifest(output, {"schemaVersion": 2})
            self.assertEqual(output.read_bytes(), original)

    def test_validate_manifest_paths_rejects_aliases(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "snapshot.sqlite3"
            source.write_bytes(b"snapshot")
            hard_link = root / "snapshot-link.sqlite3"
            os.link(source, hard_link)

            with self.assertRaisesRegex(ValueError, "source and manifest output"):
                D1_IMPORT.validate_manifest_paths(source, hard_link, None)

            shared_manifest = root / "manifest.json"
            with self.assertRaisesRegex(
                ValueError, "manifest output and comparison manifest"
            ):
                D1_IMPORT.validate_manifest_paths(
                    source, shared_manifest, shared_manifest
                )

    def test_process_manifest_files_compares_before_writing(self) -> None:
        args = argparse.Namespace(
            compare_manifest=Path("previous.json"),
            manifest_out=Path("current.json"),
        )
        calls: list[str] = []
        with mock.patch.object(
            D1_IMPORT,
            "report_manifest_diff",
            side_effect=lambda *_: calls.append("compare"),
        ), mock.patch.object(
            D1_IMPORT,
            "write_manifest",
            side_effect=lambda *_: calls.append("write"),
        ):
            D1_IMPORT.process_manifest_files(
                args, {"sourceTableCounts": {}}, {"sourceTableCounts": {}}
            )

        self.assertEqual(calls, ["compare", "write"])

    def test_process_manifest_files_does_not_write_when_comparison_fails(self) -> None:
        args = argparse.Namespace(
            compare_manifest=Path("missing.json"),
            manifest_out=Path("current.json"),
        )
        with mock.patch.object(
            D1_IMPORT,
            "report_manifest_diff",
            side_effect=ValueError("invalid comparison manifest"),
        ), mock.patch.object(D1_IMPORT, "write_manifest") as write_manifest:
            with self.assertRaisesRegex(ValueError, "invalid comparison manifest"):
                D1_IMPORT.process_manifest_files(
                    args, {"sourceTableCounts": {}}, {"sourceTableCounts": {}}
                )

        write_manifest.assert_not_called()

    def test_snapshot_completeness_requires_a_baseline_or_explicit_approval(self) -> None:
        manifest = {"sourceTableCounts": self._counts()}
        with self.assertRaisesRegex(ValueError, "comparison manifest is required"):
            D1_IMPORT.validate_snapshot_completeness(
                self._approval_args(), manifest, None
            )

        validation = D1_IMPORT.validate_snapshot_completeness(
            self._approval_args(
                approve_unbaselined_snapshot=True,
                approval_reason="Reviewed first trusted backup snapshot",
            ),
            manifest,
            None,
        )
        self.assertEqual(validation["approvalMode"], "unbaselined")
        self.assertTrue(validation["completenessApproved"])
        self.assertEqual(
            set(validation["approvedCoreCounts"]), set(D1_IMPORT.ESSENTIAL_TABLES)
        )
        self.assertEqual(
            set(validation["approvedLegacyCourseCounts"]),
            set(D1_IMPORT.LEGACY_COURSE_TABLES),
        )

    def test_snapshot_completeness_never_approves_empty_essential_tables(self) -> None:
        counts = self._counts()
        counts["teacher_timeslots"] = 0
        with self.assertRaisesRegex(ValueError, "teacher_timeslots"):
            D1_IMPORT.validate_snapshot_completeness(
                self._approval_args(
                    approve_unbaselined_snapshot=True,
                    approval_reason="Reviewed first trusted backup snapshot",
                ),
                {"sourceTableCounts": counts},
                None,
            )

        counts = self._counts()
        counts["course_aliases"] = 0
        with self.assertRaisesRegex(ValueError, "course_aliases"):
            D1_IMPORT.validate_snapshot_completeness(
                self._approval_args(
                    approve_unbaselined_snapshot=True,
                    approval_reason="Reviewed first trusted backup snapshot",
                ),
                {"sourceTableCounts": counts},
                None,
            )

    def test_table_count_validation_rejects_missing_keys(self) -> None:
        counts = self._counts()
        counts.pop("teacher")
        with self.assertRaisesRegex(ValueError, f"exactly the {len(D1_IMPORT.TABLES)}"):
            D1_IMPORT.validated_table_counts(counts, "fixture")

    def test_import_operator_label_is_bounded_and_excludes_direct_identifiers(self) -> None:
        self.assertEqual(
            D1_IMPORT.validated_operator_label(" selection-import:on-call "),
            "selection-import:on-call",
        )
        for invalid in ("xy", "Jane Doe", "person@tongji.edu.cn", "UPPERCASE", "a" * 65):
            with self.subTest(invalid=invalid):
                with self.assertRaisesRegex(ValueError, "role/service label"):
                    D1_IMPORT.validated_operator_label(invalid)

    def test_count_decrease_requires_bounded_explicit_approval(self) -> None:
        current_counts = self._counts(10)
        previous_counts = self._counts(10)
        previous_counts["teacher"] = 11
        manifest = {"sourceTableCounts": current_counts}
        baseline = {
            "snapshotSha256": "a" * 64,
            "sourceTableCounts": previous_counts,
        }
        with self.assertRaisesRegex(ValueError, "teacher"):
            D1_IMPORT.validate_snapshot_completeness(
                self._approval_args(), manifest, baseline
            )
        with self.assertRaisesRegex(ValueError, "approval reason"):
            D1_IMPORT.validate_snapshot_completeness(
                self._approval_args(
                    approve_count_decrease=True,
                    approval_reason="short",
                ),
                manifest,
                baseline,
            )

        validation = D1_IMPORT.validate_snapshot_completeness(
            self._approval_args(
                approve_count_decrease=True,
                approval_reason="Reviewed expected upstream teacher count decrease",
            ),
            manifest,
            baseline,
        )
        self.assertEqual(validation["approvalMode"], "countDecreaseOverride")
        self.assertEqual(
            validation["countDecreases"]["teacher"],
            {"before": 11, "after": 10},
        )

        current_counts = self._counts(10)
        previous_counts = self._counts(10)
        previous_counts["courses"] = 11
        validation = D1_IMPORT.validate_snapshot_completeness(
            self._approval_args(
                approve_count_decrease=True,
                approval_reason="Reviewed expected legacy course count decrease",
            ),
            {"sourceTableCounts": current_counts},
            {
                "snapshotSha256": "b" * 64,
                "sourceTableCounts": previous_counts,
            },
        )
        self.assertEqual(validation["approvalMode"], "baselineCompared")
        self.assertEqual(
            validation["legacyCourseCountDecreases"]["courses"],
            {"before": 11, "after": 10},
        )

    def test_comparison_manifest_requires_schema_source_hash_and_all_counts(self) -> None:
        current = {"sourceDatabase": D1_IMPORT.BACKUP_DATABASE_NAME}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "baseline.json"
            path.write_text(
                json.dumps(
                    {
                        "schemaVersion": D1_IMPORT.MANIFEST_SCHEMA_VERSION,
                        "sourceDatabase": D1_IMPORT.BACKUP_DATABASE_NAME,
                        "snapshotSha256": "a" * 64,
                        "sourceTableCounts": {"calendar": 1},
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                ValueError, f"exactly the {len(D1_IMPORT.TABLES)}"
            ):
                D1_IMPORT.load_comparison_manifest(path, current)


if __name__ == "__main__":
    unittest.main()
