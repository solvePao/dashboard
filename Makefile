.DEFAULT_GOAL := help

# Ensure the Rust toolchain is on PATH for wrangler's worker-build step,
# even when make runs in a shell that never sourced ~/.cargo/env.
export PATH := $(HOME)/.cargo/bin:$(PATH)

# Suppress Vite native config loader deprecation warning for Vite config files
export VITE_CONFIG_NATIVE_IGNORE_WARNING := true

NODE_REQUIRED_MAJOR := 24

.PHONY: help start-api deploy-api test-api test-coverage db-schema-local db-schema-prod \
        start-web start-mobile web mobile start-web-mobile dev dev-all \
        check-node-pnpm install-frontend install-mobile

help: ## Display this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-18s\033[0m %s\n", $$1, $$2}'

check-node-pnpm:
	@command -v node >/dev/null 2>&1 || { echo "\033[31mError: Node.js is not installed.\033[0m"; exit 1; }
	@NODE_VER=$$(node -v | sed 's/^v//' | cut -d. -f1); \
	if [ "$$NODE_VER" != "$(NODE_REQUIRED_MAJOR)" ]; then \
		echo "\033[31mError: Node.js v$(NODE_REQUIRED_MAJOR).x is required (found $$(node -v)).\033[0m"; \
		exit 1; \
	fi
	@command -v pnpm >/dev/null 2>&1 || { echo "\033[31mError: pnpm is not installed. Please install pnpm (e.g. brew install pnpm or npm i -g pnpm).\033[0m"; exit 1; }

install-frontend: check-node-pnpm
	@if [ ! -f "frontend/package.json" ]; then \
		echo "\033[31mError: frontend/package.json not found.\033[0m"; \
		exit 1; \
	fi
	@if [ ! -d "frontend/node_modules" ] || [ ! -f "frontend/node_modules/.bin/vite" ]; then \
		echo "Installing frontend dependencies with pnpm..."; \
		(cd frontend && pnpm install); \
	fi

install-mobile: check-node-pnpm
	@if [ ! -f "mobile/package.json" ]; then \
		echo "\033[31mError: mobile/package.json not found.\033[0m"; \
		exit 1; \
	fi
	@if [ ! -d "mobile/node_modules" ] || [ ! -f "mobile/node_modules/.bin/vite" ]; then \
		echo "Installing mobile dependencies with pnpm..."; \
		(cd mobile && pnpm install); \
	fi

start-api: ## Run backend worker locally (local SQLite D1, never remote)
	cd backend && npx wrangler dev --local

deploy-api: ## Deploy backend worker to production (remote D1)
	cd backend && npx wrangler deploy --env production

test-api: ## Run backend Rust tests in parallel (host, mocked D1/Google)
	cd backend && cargo test --tests -- --test-threads=8

test-coverage: ## Run backend Rust tests and enforce 100% coverage
	cd backend && cargo llvm-cov --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100

db-schema-local: ## Apply backend/schema.sql to the local D1 (SQLite) database
	cd backend && npx wrangler d1 execute dashboard --local --file=./schema.sql

db-schema-prod: ## Apply backend/schema.sql to the PRODUCTION D1 database (remote)
	cd backend && npx wrangler d1 execute dashboard --remote --file=./schema.sql

start-web: install-frontend ## Run frontend server
	cd frontend && pnpm dev

start-mobile: install-mobile ## Run mobile frontend server
	cd mobile && pnpm dev

web: start-web ## Alias for start-web

mobile: start-mobile ## Alias for start-mobile

start-web-mobile: install-frontend install-mobile ## Run web and mobile servers concurrently
	@trap 'kill 0' EXIT; (cd frontend && pnpm dev) & (cd mobile && pnpm dev)

dev: install-frontend ## Run backend and frontend concurrently
	@trap 'kill 0' EXIT; (cd backend && npx wrangler dev --local) & (cd frontend && pnpm dev)

dev-all: install-frontend install-mobile ## Run backend, frontend, and mobile concurrently
	@trap 'kill 0' EXIT; (cd backend && npx wrangler dev --local) & (cd frontend && pnpm dev) & (cd mobile && pnpm dev)
