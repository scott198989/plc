from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "apps" / "windows-shell" / "src" / "main.cpp"


class WindowsShellWebViewPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SOURCE.read_text(encoding="utf-8")

    def test_browser_runtime_network_services_fail_closed(self) -> None:
        for required in (
            "--disable-background-networking",
            "--disable-client-side-phishing-detection",
            '--host-resolver-rules=\\"MAP * ~NOTFOUND\\"',
            "--no-proxy-server",
            "msEdgeAffiliationBackend",
            "msSmartScreenProtection",
        ):
            self.assertIn(required, self.source)

    def test_credential_backends_are_disabled(self) -> None:
        settings = self.source.index("ComPtr<ICoreWebView2Settings4> settings4;")
        password = self.source.index(
            "settings4->put_IsPasswordAutosaveEnabled(FALSE)", settings
        )
        autofill = self.source.index(
            "settings4->put_IsGeneralAutofillEnabled(FALSE)", password
        )
        reputation = self.source.index(
            "ComPtr<ICoreWebView2Settings8> settings8;", autofill
        )
        self.assertLess(settings, password)
        self.assertLess(password, autofill)
        self.assertLess(autofill, reputation)

    def test_supported_smartscreen_setting_is_disabled_before_navigation(self) -> None:
        settings = self.source.index("ComPtr<ICoreWebView2Settings8> settings8;")
        disabled = self.source.index(
            "settings8->put_IsReputationCheckingRequired(FALSE)", settings
        )
        navigation = self.source.index("webview_->Navigate(kApplicationUri)", disabled)
        self.assertLess(settings, disabled)
        self.assertLess(disabled, navigation)

    def test_only_packaged_virtual_origin_is_admitted(self) -> None:
        navigation_handler = self.source.index(
            "ICoreWebView2NavigationStartingEventHandler"
        )
        resource_handler = self.source.index("HRESULT on_resource_requested(")
        policy_slice = self.source[navigation_handler:resource_handler]
        self.assertIn("std::wstring_view(uri) == kApplicationUri", policy_slice)
        self.assertIn("args->put_Cancel(allowed ? FALSE : TRUE)", policy_slice)


if __name__ == "__main__":
    unittest.main()
