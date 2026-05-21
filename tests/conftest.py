"""
Shared fixtures for norse integration tests.

Tests interact with the binary through two external surfaces:
  - NORSE_GITHUB_API env var → points the binary at our mock HTTP server
  - tmux session            → drives keyboard input, captures rendered screen

The binary path is controlled by the NORSE_BINARY env var (defaults to the
debug build). Tests are language-agnostic: replace the binary with a
rewrite in any language and they still pass.
"""

import os
import re
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

import pytest

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).parent.parent
BINARY = os.environ.get("NORSE_BINARY", str(REPO_ROOT / "target" / "debug" / "terminal"))
FIXTURES = Path(__file__).parent / "fixtures"


# ---------------------------------------------------------------------------
# Mock GitHub API server
# ---------------------------------------------------------------------------

class _Handler(BaseHTTPRequestHandler):
    base_url: str = ""  # set after server binds

    def log_message(self, *_):
        pass  # silence access log

    def _respond(self, status: int, body: bytes, content_type: str = "application/json"):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _fixture(self, name: str) -> bytes:
        return (FIXTURES / name).read_bytes()

    def _fixture_with_base(self, name: str) -> bytes:
        raw = (FIXTURES / name).read_text()
        return raw.replace("BASE_URL", self.__class__.base_url).encode()

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        qs = parse_qs(parsed.query)
        accept = self.headers.get("Accept", "")

        if path == "/search/repositories":
            self._respond(200, self._fixture("repos_search.json"))

        elif path == "/search/issues":
            q = qs.get("q", [""])[0]
            if "review-requested" in q:
                self._respond(200, self._fixture_with_base("prs_reviews.json"))
            else:
                self._respond(200, self._fixture_with_base("prs_org.json"))

        elif path == "/user/subscriptions":
            self._respond(200, self._fixture("watched_repos.json"))

        # /repos/{owner}/{repo}/pulls/{number} — must be before commits route
        elif re.match(r"^/repos/[^/]+/[^/]+/pulls/\d+$", path):
            if "diff" in accept:
                self._respond(200, self._fixture("pr_diff.txt"), "text/x-diff")
            else:
                self._respond(200, b'{"html_url":"https://github.com/test-org/alpha-repo/pull/42"}')

        # /repos/{owner}/{repo}/commits/{sha}/pulls
        elif re.match(r"^/repos/[^/]+/[^/]+/commits/[^/]+/pulls$", path):
            self._respond(200, b'[{"html_url":"https://github.com/test-org/alpha-repo/pull/42"}]')

        # /repos/{owner}/{repo}/commits/{sha} — single commit diff
        elif re.match(r"^/repos/[^/]+/[^/]+/commits/[^/]+$", path):
            if "diff" in accept:
                self._respond(200, self._fixture("commit_diff.txt"), "text/x-diff")
            else:
                self._respond(200, b'{}')

        elif re.match(r"^/repos/[^/]+/[^/]+/languages$", path):
            self._respond(200, self._fixture("languages.json"))

        elif re.match(r"^/repos/[^/]+/[^/]+/commits$", path):
            self._respond(200, self._fixture("commits.json"))

        elif re.match(r"^/repos/[^/]+/[^/]+/subscription$", path):
            self._respond(200, b'{}')

        else:
            self._respond(404, b'{"message":"Not Found"}')

    def do_PUT(self):
        self._respond(200, b'{}')

    def do_DELETE(self):
        self._respond(204, b'')


@pytest.fixture(scope="session")
def mock_server():
    """Session-scoped mock GitHub API. Returns its base URL."""
    server = HTTPServer(("127.0.0.1", 0), _Handler)
    port = server.server_address[1]
    base = f"http://127.0.0.1:{port}"
    _Handler.base_url = base

    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    yield base
    server.shutdown()


# ---------------------------------------------------------------------------
# tmux helpers (module-level so tests can import them directly)
# ---------------------------------------------------------------------------

def screen(session: str) -> str:
    """Return the current rendered text of a tmux pane (ANSI stripped)."""
    r = subprocess.run(
        ["tmux", "capture-pane", "-t", session, "-p", "-J"],
        capture_output=True, text=True,
    )
    return r.stdout


def send_key(session: str, key: str):
    """
    Send one key to the tmux session and wait one event-loop tick.

    Use tmux key names: single chars ('p', 'j', 'q'), 'Enter', 'Escape',
    'Up', 'Down', 'C-x', 'C-h', 'C-l', 'C-d', 'C-u', etc.
    """
    subprocess.run(["tmux", "send-keys", "-t", session, key], check=True)
    time.sleep(0.12)  # binary polls every 50 ms; two ticks of margin


def wait_for(session: str, text: str, timeout: float = 4.0) -> bool:
    """Poll the screen until *text* appears or *timeout* seconds elapse."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if text in screen(session):
            return True
        time.sleep(0.1)
    return False


def assert_screen(session: str, text: str, timeout: float = 4.0):
    """Assert *text* appears on screen within *timeout* seconds."""
    assert wait_for(session, text, timeout), (
        f"Timed out waiting for {text!r}\n\nScreen:\n{screen(session)}"
    )


# ---------------------------------------------------------------------------
# Per-test norse session fixture
# ---------------------------------------------------------------------------

@pytest.fixture
def norse(mock_server, tmp_path):
    """
    Start a fresh norse process inside a tmux session.

    - Writes a minimal config.toml in a temp dir
    - Points NORSE_GITHUB_API at the mock server
    - Sets HOME to a writable tmp dir so the LLM client silently fails
      (no ~/.claude/settings.json → AI features disabled, everything else works)
    - Yields the tmux session name
    - Kills the session on teardown
    """
    # Check prerequisites
    if not Path(BINARY).exists():
        pytest.skip(f"binary not found: {BINARY}  (run `cargo build` first)")
    if subprocess.run(["which", "tmux"], capture_output=True).returncode != 0:
        pytest.skip("tmux is not installed")

    session = f"norse-{os.getpid()}-{int(time.time() * 1000) % 100000}"

    # fake HOME: no claude settings → no LLM, no side effects
    fake_home = tmp_path / "home"
    fake_home.mkdir()

    (fake_home / ".norse").write_text(
        '[github]\ntoken = "test-token"\norgs = ["test-org"]\n'
    )

    subprocess.run(
        ["tmux", "new-session", "-d", "-s", session, "-x", "220", "-y", "50"],
        check=True,
    )

    launch = (
        f"HOME={fake_home} "
        f"NORSE_GITHUB_API={mock_server} "
        f"{BINARY}"
    )
    subprocess.run(["tmux", "send-keys", "-t", session, launch, "Enter"], check=True)

    # Wait for the TUI to render its first frame ("source code tools" is the
    # subtitle only rendered by the app itself, not by the shell command echo)
    assert wait_for(session, "source code tools", timeout=8.0), (
        f"TUI never started.\nScreen:\n{screen(session)}"
    )

    yield session

    subprocess.run(["tmux", "kill-session", "-t", session], check=False)
