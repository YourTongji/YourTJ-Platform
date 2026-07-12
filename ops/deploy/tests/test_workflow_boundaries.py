from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).parents[3]


class WorkflowBoundaryTests(unittest.TestCase):
    def test_main_deployment_uses_versioned_script_and_main_ref_gate(self):
        workflow = (ROOT / ".github/workflows/deploy-main.yml").read_text()
        self.assertIn("ops/deploy/deploy-main.sh", workflow)
        self.assertIn("github.ref == 'refs/heads/main'", workflow)
        self.assertNotIn("/opt/yourtj-preview/deploy-main.sh", workflow)

    def test_preview_never_references_production_oss_secrets(self):
        workflow = (ROOT / ".github/workflows/pr-preview.yml").read_text()
        for key in (
            "OSS_ACCESS_KEY_ID",
            "OSS_ACCESS_KEY_SECRET",
            "OSS_BUCKET",
            "OSS_ROLE_ARN",
        ):
            self.assertNotIn(key, workflow)


if __name__ == "__main__":
    unittest.main()
