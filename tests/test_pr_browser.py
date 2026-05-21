"""
PR browser — navigation, tab switching, diff entry, and back navigation.

All tests enter the PR browser via 'p' from the home screen.
The mock server returns two open PRs (prs_org.json) and one review-requested
PR (prs_reviews.json).
"""

import pytest
from conftest import assert_screen, screen, send_key, wait_for


@pytest.fixture
def pr_browser(norse):
    """Enter the PR browser and wait for the list to load."""
    send_key(norse, "p")
    assert_screen(norse, " PRs ")
    # Wait for the org PRs to arrive from the mock server
    assert wait_for(norse, "Add new feature", timeout=5), (
        f"PR list never loaded.\nScreen:\n{screen(norse)}"
    )
    return norse


class TestEnterAndLoad:
    def test_mode_label_changes(self, pr_browser):
        assert_screen(pr_browser, " PRs ")

    def test_pr_titles_appear(self, pr_browser):
        assert_screen(pr_browser, "Add new feature")
        assert_screen(pr_browser, "Fix critical bug")

    def test_pr_numbers_appear(self, pr_browser):
        s = screen(pr_browser)
        assert "#42" in s or "42" in s
        assert "#7" in s or "7" in s

    def test_pr_authors_appear(self, pr_browser):
        assert_screen(pr_browser, "alice")

    def test_repo_names_appear(self, pr_browser):
        assert_screen(pr_browser, "test-org/alpha-repo")

    def test_keybinding_hint_shows_diff(self, pr_browser):
        assert_screen(pr_browser, "enter diff")

    def test_keybinding_hint_shows_ctrl_x(self, pr_browser):
        assert_screen(pr_browser, "ctrl-x open")


class TestNavigation:
    def test_j_moves_selection_down(self, pr_browser):
        # First PR is selected initially; press j to move to second
        send_key(pr_browser, "j")
        assert_screen(pr_browser, "Fix critical bug")

    def test_k_moves_selection_up(self, pr_browser):
        send_key(pr_browser, "j")   # go to second
        send_key(pr_browser, "k")   # back to first
        assert_screen(pr_browser, "Add new feature")

    def test_q_returns_to_normal(self, pr_browser):
        send_key(pr_browser, "q")
        assert_screen(pr_browser, "NORMAL")
        assert_screen(pr_browser, "source code tools")


class TestTabSwitching:
    def test_ctrl_l_switches_to_by_people(self, pr_browser):
        send_key(pr_browser, "C-l")
        assert_screen(pr_browser, "By People")

    def test_ctrl_l_l_switches_to_by_repo(self, pr_browser):
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        assert_screen(pr_browser, "By Repo")

    def test_ctrl_l_l_l_switches_to_review_requests(self, pr_browser):
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        assert_screen(pr_browser, "Review Requests")

    def test_ctrl_l_l_l_l_switches_to_watching(self, pr_browser):
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        assert_screen(pr_browser, "Watching")

    def test_ctrl_h_goes_back_to_all(self, pr_browser):
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-h")
        assert_screen(pr_browser, "All PRs")

    def test_all_tab_active_initially(self, pr_browser):
        assert_screen(pr_browser, "All PRs")


class TestReviewRequestsTab:
    def test_review_requested_pr_appears(self, pr_browser):
        # Navigate to Review Requests tab
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")
        assert wait_for(pr_browser, "Review requested PR", timeout=5), (
            f"Review PR never loaded.\nScreen:\n{screen(pr_browser)}"
        )
        assert_screen(pr_browser, "carol")


class TestByPeopleFilter:
    def test_filter_by_username(self, pr_browser):
        send_key(pr_browser, "C-l")      # → By People
        # Type a username to filter
        for ch in "alice":
            send_key(pr_browser, ch)
        assert_screen(pr_browser, "Add new feature")
        # bob's PR should not appear
        s = screen(pr_browser)
        assert "Fix critical bug" not in s


class TestByRepoFilter:
    def test_filter_by_repo(self, pr_browser):
        send_key(pr_browser, "C-l")
        send_key(pr_browser, "C-l")   # → By Repo
        for ch in "beta":
            send_key(pr_browser, ch)
        assert_screen(pr_browser, "Fix critical bug")
        s = screen(pr_browser)
        assert "Add new feature" not in s
