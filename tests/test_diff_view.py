"""
Diff view — entered from the PR browser by pressing Enter on a PR.

The mock server returns pr_diff.txt for any PR diff request.
Tests cover: content rendering, navigation, back-button behavior,
and the header format.

Note: the diff view takes over the full terminal; there is no shared status
bar showing "DIFF".  Presence in the diff view is detected by the bottom
hint "ctrl-x open PR" rendered inside the diff block border.
"""

import pytest
from conftest import assert_screen, screen, send_key, wait_for

# Unique text rendered only when the diff block is on screen
_DIFF_SENTINEL = "ctrl-x open PR"


@pytest.fixture
def diff_view(norse):
    """Enter PR browser, load PRs, open the first PR's diff."""
    send_key(norse, "p")
    assert wait_for(norse, "Add new feature", timeout=5), (
        f"PR list never loaded.\nScreen:\n{screen(norse)}"
    )
    send_key(norse, "Enter")
    assert wait_for(norse, _DIFF_SENTINEL, timeout=5), (
        f"Diff view never opened.\nScreen:\n{screen(norse)}"
    )
    return norse


class TestDiffViewEntry:
    def test_diff_controls_shown(self, diff_view):
        """The diff block's bottom hint is visible — we are in the diff view."""
        assert_screen(diff_view, _DIFF_SENTINEL)

    def test_header_shows_repo(self, diff_view):
        assert_screen(diff_view, "test-org/alpha-repo")

    def test_header_shows_pr_number(self, diff_view):
        assert_screen(diff_view, "#42")

    def test_header_shows_pr_title(self, diff_view):
        assert_screen(diff_view, "Add new feature")


class TestDiffContent:
    def test_diff_lines_appear(self, diff_view):
        assert wait_for(diff_view, "src/main.rs", timeout=5), (
            f"Diff content never loaded.\nScreen:\n{screen(diff_view)}"
        )

    def test_added_line_visible(self, diff_view):
        assert wait_for(diff_view, "hello world", timeout=5)

    def test_diff_chunk_header_visible(self, diff_view):
        assert wait_for(diff_view, "@@", timeout=5)


class TestDiffNavigation:
    def test_j_scrolls_down(self, diff_view):
        send_key(diff_view, "j")
        assert_screen(diff_view, _DIFF_SENTINEL)

    def test_k_scrolls_up(self, diff_view):
        send_key(diff_view, "j")
        send_key(diff_view, "k")
        assert_screen(diff_view, _DIFF_SENTINEL)

    def test_ctrl_d_page_down(self, diff_view):
        send_key(diff_view, "C-d")
        assert_screen(diff_view, _DIFF_SENTINEL)

    def test_ctrl_u_page_up(self, diff_view):
        send_key(diff_view, "C-d")
        send_key(diff_view, "C-u")
        assert_screen(diff_view, _DIFF_SENTINEL)


class TestDiffBackNavigation:
    def test_q_returns_to_pr_browser(self, diff_view):
        """Pressing q in a PR diff should return to the PR browser, not home."""
        send_key(diff_view, "q")
        assert_screen(diff_view, " PRs ")
        s = screen(diff_view)
        assert "NORMAL" not in s

    def test_escape_returns_to_pr_browser(self, diff_view):
        send_key(diff_view, "Escape")
        assert_screen(diff_view, " PRs ")

    def test_can_re_enter_diff_after_back(self, diff_view):
        send_key(diff_view, "q")
        assert_screen(diff_view, " PRs ")
        send_key(diff_view, "Enter")
        assert_screen(diff_view, _DIFF_SENTINEL)


class TestDiffKeybindingHints:
    def test_statusbar_shows_ctrl_x_hint(self, diff_view):
        assert_screen(diff_view, "ctrl-x")

    def test_statusbar_shows_navigation_hint(self, diff_view):
        s = screen(diff_view)
        assert "j/k" in s
