from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).parents[3]


class WorkflowBoundaryTests(unittest.TestCase):
    def test_main_deployment_uses_versioned_script_and_main_ref_gate(self):
        workflow = (ROOT / ".github/workflows/deploy-main.yml").read_text()
        deploy = (ROOT / "ops/deploy/deploy-main.sh").read_text()
        self.assertIn("ops/deploy/deploy-main.sh", workflow)
        self.assertIn("github.ref == 'refs/heads/main'", workflow)
        self.assertNotIn("/opt/yourtj-preview/deploy-main.sh", workflow)
        self.assertIn("cancel-in-progress: false", workflow)
        self.assertIn("MAIN_PUBLIC_BASE_URL", workflow)
        self.assertIn("verify canonical public HTTPS routes", workflow)
        self.assertIn("verify shared-host private ports are not public", workflow)
        self.assertIn("range(15000, 17000)", workflow)
        self.assertIn("quarantine_unsafe_preview_containers", deploy)
        self.assertIn("ops/deploy/frontend-nginx.conf.template", workflow)
        self.assertIn("ops/deploy/preview-proxy.conf", workflow)
        self.assertIn("${GITHUB_SHA}-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}", workflow)
        self.assertIn("immutable main release", deploy)
        self.assertIn('BIND_ADDRESS=127.0.0.1', deploy)
        self.assertIn('-p "127.0.0.1:${FRONTEND_PORT}:80"', deploy)
        self.assertIn("--enforce-controlled-wallet-migration", deploy)
        self.assertIn("--wallet-key-cutover-drained", deploy)
        self.assertIn("WALLET_KEY_CUTOVER_DRAIN_SECONDS=360", deploy)
        self.assertIn("WALLET_KEY_CUTOVER_APPROVED_REVISION", workflow)
        self.assertIn(
            'wallet_cutover_approval="${WALLET_KEY_CUTOVER_APPROVED_REVISION:-not-approved}"',
            workflow,
        )
        self.assertIn('"$wallet_cutover_approval" =~ ^[0-9a-f]{40}$', workflow)
        self.assertIn('"${wallet_cutover_approval}"', workflow)

    def test_preview_never_references_production_oss_secrets(self):
        workflow = (ROOT / ".github/workflows/pr-preview.yml").read_text()
        for key in (
            "OSS_ACCESS_KEY_ID",
            "OSS_ACCESS_KEY_SECRET",
            "OSS_BUCKET",
            "OSS_ROLE_ARN",
        ):
            self.assertNotIn(key, workflow)

    def test_frontend_csp_separates_ingest_uploads_from_cdn_reads(self):
        template = (ROOT / "ops/deploy/frontend-nginx.conf.template").read_text()
        main_deploy = (ROOT / "ops/deploy/deploy-main.sh").read_text()
        preview_deploy = (ROOT / "ops/deploy/deploy-pr.sh").read_text()
        self.assertIn("connect-src 'self' https://captcha.07211024.xyz __MEDIA_INGEST_ORIGIN__", template)
        self.assertIn("img-src 'self' data: blob: https://captcha.07211024.xyz __MEDIA_CDN_ORIGIN__", template)
        self.assertIn('https://${OSS_BUCKET}.oss-${OSS_REGION}.aliyuncs.com', main_deploy)
        self.assertIn("https://ingest.invalid", preview_deploy)
        self.assertIn("https://media.invalid", preview_deploy)

    def test_preview_cleanup_is_versioned_and_contains_no_password_literal(self):
        workflow = (ROOT / ".github/workflows/pr-preview.yml").read_text()
        cleanup = (ROOT / "ops/deploy/cleanup-pr.sh").read_text()
        deploy = (ROOT / "ops/deploy/deploy-pr.sh").read_text()
        self.assertIn("ops/deploy/cleanup-pr.sh", workflow)
        self.assertIn("ops/deploy/deploy-pr.sh", workflow)
        self.assertIn("if: github.event.action == 'closed'", workflow)
        self.assertNotIn("github.event.pull_request.merged == false", workflow)
        self.assertNotIn("/opt/yourtj-preview/deploy-pr.sh", workflow)
        self.assertNotIn("PGPASSWORD", workflow)
        self.assertNotIn("preview_pass", workflow)
        self.assertIn("${HOME}/.pgpass", cleanup)
        self.assertIn("PGPASSFILE", cleanup)
        self.assertNotIn("PGPASSWORD", cleanup)
        self.assertIn("PGPASSFILE", deploy)
        self.assertNotIn("preview_pass", deploy)
        self.assertNotIn("pr-${PR_NUMBER}-preview-secret", deploy)
        self.assertIn("backend_new_started=0", deploy)
        self.assertIn("frontend_new_started=0", deploy)
        self.assertIn("if ((backend_new_started == 1))", deploy)
        self.assertIn("if ((frontend_new_started == 1))", deploy)
        self.assertIn("forward-only schema cutover", deploy)
        self.assertIn("docker create", deploy)
        self.assertIn('docker start "$BACKEND_CONTAINER"', deploy)
        started_marker = deploy.index("backend_new_started=1")
        self.assertLess(
            started_marker,
            deploy.index('docker start "$BACKEND_CONTAINER"', started_marker),
        )
        self.assertNotIn('docker rm -f "$BACKEND_CONTAINER"', deploy)
        self.assertIn("preview backend revision label mismatch", deploy)
        self.assertIn('BIND_ADDRESS=127.0.0.1', deploy)
        self.assertIn('-p "127.0.0.1:${FRONTEND_PORT}:80"', deploy)
        self.assertIn("/releases/${RELEASE}/frontend", workflow)
        self.assertIn("immutable PR release", deploy)
        self.assertNotIn("rm -rf ${FE_DIR}/*", workflow)
        self.assertIn("verify preview container ports are not public", workflow)

    def test_preview_and_proxy_share_the_same_three_digit_port_scheme(self):
        workflow = (ROOT / ".github/workflows/pr-preview.yml").read_text()
        deploy = (ROOT / "ops/deploy/deploy-pr.sh").read_text()
        proxy = (ROOT / "ops/deploy/preview-proxy.conf").read_text()
        self.assertIn("^[1-9][0-9]{0,2}$", workflow)
        self.assertIn("PR number must be between 1 and 999", deploy)
        self.assertIn("~^[1-9][0-9][0-9]$ 15$pr_number", proxy)
        self.assertIn("~^[1-9][0-9][0-9]$ 16$pr_number", proxy)
        self.assertNotIn("{0,2}", proxy)
        self.assertNotIn("{2}", proxy)
        self.assertIn("real_ip_header CF-Connecting-IP", proxy)
        self.assertIn("proxy_set_header X-Forwarded-For $remote_addr", proxy)
        self.assertNotIn("$proxy_add_x_forwarded_for", proxy)

    def test_preview_deployment_files_trigger_a_preview_build(self):
        workflow = (ROOT / ".github/workflows/pr-preview.yml").read_text()
        for path in (
            "ops/deploy/deploy-pr.sh",
            "ops/deploy/cleanup-pr.sh",
            "ops/deploy/frontend-nginx.conf.template",
        ):
            self.assertGreaterEqual(workflow.count(path), 2)


if __name__ == "__main__":
    unittest.main()
