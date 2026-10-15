.DEFAULT_GOAL := help

# Ensure the Rust toolchain is on PATH for wrangler's worker-build step,
# even when make runs in a shell that never sourced ~/.cargo/env.
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: help start-api test-api start-web start-mobile dev

help: ## Display this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

start-api: ## Run backend worker locally (local SQLite D1, never remote)
	cd backend && npx wrangler dev --local

deploy-api: ## Deploy backend worker to production (remote D1)
	cd backend && npx wrangler deploy --env production

test-api: ## Run backend Rust tests in parallel (host, mocked D1/Google)
	cd backend && cargo test --tests -- --test-threads=8

db-schema-local: ## Apply backend/schema.sql to the local D1 (SQLite) database
	cd backend && npx wrangler d1 execute dashboard --local --file=./schema.sql

db-schema-prod: ## Apply backend/schema.sql to the PRODUCTION D1 database (remote)
	cd backend && npx wrangler d1 execute dashboard --remote --file=./schema.sql

start-web: ## Run frontend server
	npm run dev --prefix frontend

start-mobile: ## Run mobile frontend server
	npm run dev --prefix mobile

dev: ## Run backend and frontend concurrently
	@trap 'kill 0' EXIT; (cd backend && npx wrangler dev) & npm run dev --prefix frontend
