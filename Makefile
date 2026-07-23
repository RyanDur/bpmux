# bpmux pre-push gates, addressable as named targets so Ryan + agents
# can run individual stages locally without invoking the git hook.
# Mirrors `scripts/git-hooks/pre-push` post-slice-29 (cargo runs from
# $REPO_ROOT, no temp worktree). Targets are PHONY — no file outputs
# to track; each target re-runs its underlying tool every invocation.

.PHONY: help cargo-test audit npm-unit npm-e2e pre-push push check

help:
	@echo "make cargo-test   # cargo nextest run --no-fail-fast --retries 1"
	@echo "make audit        # cargo audit --deny warnings"
	@echo "make npm-unit     # applications/electron unit tests"
	@echo "make npm-e2e      # applications/electron Playwright e2e"
	@echo "make pre-push     # full hook: fmt + clippy + nextest + electron build/typecheck + npm-unit + lint + npm-e2e + file-size"

cargo-test:
	cargo nextest run --no-fail-fast --retries 1

audit:
	cargo audit --deny warnings

npm-unit:
	cd applications/electron && npm run test:unit

npm-e2e:
	cd applications/electron && npm run test:e2e

check:
ifndef T
	@echo "usage: make check T=<pattern>   (nextest filter AND/OR vitest path fragment)"
else
	env -u BPMUX_PORT -u BPMUX_TERMINAL_ID -u BPMUX_USER_ID -u BPMUX_ROLE -u BPMUX_DATA_BASE cargo nextest run -E 'test(/$(T)/)' --no-fail-fast || true
	cd applications/electron && env -u BPMUX_PORT -u BPMUX_TERMINAL_ID -u BPMUX_USER_ID -u BPMUX_ROLE -u BPMUX_DATA_BASE npx vitest run --passWithNoTests "$(T)" || true
endif

push:
	git push
	scripts/verify-push.sh

pre-push:
	bash scripts/git-hooks/pre-push
