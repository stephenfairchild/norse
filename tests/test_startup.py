"""
Startup and home-screen behavior.

These tests verify what a user sees the moment the app launches and that
the most basic keybindings work.  Nothing GitHub-specific is needed here;
these pass even if the mock server returns errors.
"""

from conftest import assert_screen, screen, send_key, wait_for


class TestHomeScreen:
    def test_shows_logo(self, norse):
        # "source code tools" is the subtitle line rendered directly below the
        # ASCII art — only the TUI itself emits this text.
        assert_screen(norse, "source code tools")

    def test_shows_normal_mode(self, norse):
        assert_screen(norse, "NORMAL")

    def test_shows_configured_org(self, norse):
        assert_screen(norse, "test-org")

    def test_shows_key_hints(self, norse):
        assert_screen(norse, "search repos")
        assert_screen(norse, "browse PRs")
        assert_screen(norse, "quit")


class TestQuit:
    def test_q_quits(self, norse):
        send_key(norse, "q")
        import time; time.sleep(0.3)
        s = screen(norse)
        # After quit the TUI exits; the pane should no longer show "NORMAL"
        assert "NORMAL" not in s
