test:
	cargo test
	cd tests && python3 -m pip install -q -r requirements.txt && python3 -m pytest

release:
	cog bump --auto && git push && git push --tags

.PHONY: test release
