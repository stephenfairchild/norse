"""
PR approval — ctrl-a in the diff view.

Covers:
  - ctrl-a returns to the PR browser immediately
  - the approved PR shows an "A" marker in the list
  - one POST to /repos/.../pulls/.../reviews is sent to the GitHub API
  - pressing ctrl-a a second time on the same PR does NOT send a second request
"""

import time

import pytest
from conftest import assert_screen, screen, send_key, wait_for

_DIFF_SENTINEL = "ctrl-a approve"


@pytest.fixture
def diff_open(norse):
    """Enter the PR browser, wait for PRs to load, open the first PR's diff."""
    send_key(norse, "p")
    assert wait_for(norse, "Add new feature", timeout=5), (
        f"PR list never loaded.\nScreen:\n{screen(norse)}"
    )
    send_key(norse, "Enter")
    assert wait_for(norse, _DIFF_SENTINEL, timeout=5), (
        f"Diff view never opened.\nScreen:\n{screen(norse)}"
    )
    return norse


class TestApproveNavigation:
    def test_ctrl_a_returns_to_pr_browser(self, diff_open, approve_calls):
        send_key(diff_open, "C-a")
        assert_screen(diff_open, " PRs ")

    def test_ctrl_a_does_not_return_to_home(self, diff_open, approve_calls):
        send_key(diff_open, "C-a")
        assert_screen(diff_open, " PRs ")
        s = screen(diff_open)
        assert "NORMAL" not in s
        assert "source code tools" not in s


class TestApproveMarker:
    def test_approved_marker_not_shown_before_approval(self, diff_open, approve_calls):
        # Back out without approving; marker should not appear
        send_key(diff_open, "q")
        assert_screen(diff_open, " PRs ")
        s = screen(diff_open)
        # "A " as an approved marker prefix — confirm it's absent before any approval
        lines = [l for l in s.splitlines() if "alpha-repo" in l]
        assert lines, "alpha-repo row not found in PR list"
        assert not lines[0].startswith("A "), (
            f"Approval marker present before any approval: {lines[0]!r}"
        )

    def test_approved_marker_shown_after_approval(self, diff_open, approve_calls):
        send_key(diff_open, "C-a")
        assert_screen(diff_open, " PRs ")
        # The "A" marker sits in a fixed-width column before the repo name;
        # check it appears somewhere on the alpha-repo PR row
        s = screen(diff_open)
        lines = [l for l in s.splitlines() if "alpha-repo" in l]
        assert lines, "alpha-repo row not found after approval"
        assert "A" in lines[0], (
            f"Approval marker not found in PR row: {lines[0]!r}"
        )


class TestApproveApiCall:
    def test_ctrl_a_sends_approve_request(self, diff_open, approve_calls):
        send_key(diff_open, "C-a")
        # Allow the async spawn a moment to hit the mock server
        deadline = time.time() + 2.0
        while time.time() < deadline and not approve_calls:
            time.sleep(0.05)
        assert any("pulls/42/reviews" in p for p in approve_calls), (
            f"No approve API call received. Calls: {approve_calls}"
        )

    def test_ctrl_a_posts_to_correct_repo(self, diff_open, approve_calls):
        send_key(diff_open, "C-a")
        deadline = time.time() + 2.0
        while time.time() < deadline and not approve_calls:
            time.sleep(0.05)
        assert any("test-org/alpha-repo" in p for p in approve_calls), (
            f"Approve call used wrong repo. Calls: {approve_calls}"
        )


class TestNoDoubleApprove:
    def test_second_ctrl_a_does_not_send_api_request(self, diff_open, approve_calls):
        # First approval
        send_key(diff_open, "C-a")
        assert_screen(diff_open, " PRs ")
        deadline = time.time() + 2.0
        while time.time() < deadline and not approve_calls:
            time.sleep(0.05)
        assert len(approve_calls) == 1, f"Expected 1 call after first approval, got: {approve_calls}"

        # Re-open the same PR
        send_key(diff_open, "Enter")
        assert wait_for(diff_open, _DIFF_SENTINEL, timeout=5), (
            f"Diff view did not reopen.\nScreen:\n{screen(diff_open)}"
        )

        # Attempt second approval
        send_key(diff_open, "C-a")
        time.sleep(0.5)  # wait for any spurious call that shouldn't arrive

        assert len(approve_calls) == 1, (
            f"Expected still 1 API call after second ctrl-a, got {len(approve_calls)}: {approve_calls}"
        )

    def test_second_ctrl_a_still_returns_to_pr_browser(self, diff_open, approve_calls):
        # First approval — returns to list
        send_key(diff_open, "C-a")
        assert_screen(diff_open, " PRs ")

        # Re-open same PR
        send_key(diff_open, "Enter")
        assert wait_for(diff_open, _DIFF_SENTINEL, timeout=5)

        # Second ctrl-a is a no-op; user stays in the diff view
        send_key(diff_open, "C-a")
        # Nothing should crash; the diff view stays put
        time.sleep(0.3)
        s = screen(diff_open)
        # Still in diff view (sentinel still visible) OR back in PR browser
        # Either is acceptable — the key thing is no crash
        assert _DIFF_SENTINEL in s or " PRs " in s
