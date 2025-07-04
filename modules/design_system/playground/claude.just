# Claude Code Automation Commands
# Non-blocking commands designed for automated use

# Run all tests and show all failures
test:
    npm run test -- --reporter=list 2>/dev/null

# Run functional tests only
test-functional:
    npm run test:functional -- --reporter=list 2>/dev/null

# Quick test run - stop at first failure for immediate feedback
test-quick:
    npm run test -- --reporter=list --max-failures=1 2>/dev/null | head -15

# Run specific test file
test-file FILE:
    npm run test -- {{FILE}} --reporter=list 2>/dev/null | head -15

# Run tests in headless mode with list output
test-headless:
    npm run test -- --reporter=list --headed=false 2>/dev/null | head -20

# Visual regression test only
test-visual:
    npm run test:visual -- --reporter=list 2>/dev/null

# Update visual baselines (non-blocking)
update-visuals:
    npm run test:visual:update 2>/dev/null
    @echo "✅ Visual baselines updated"

# TypeScript compilation check
type-check:
    @npm run type-check 2>/dev/null

# Lint check (non-blocking)
lint:
    @npm run lint 2>/dev/null

# Format check (non-blocking)
format-check:
    @npm run format:check 2>/dev/null

# Auto-fix linting and formatting
lint-fix:
    @npm run lint:fix 2>/dev/null
    @npm run format 2>/dev/null

# Clean test artifacts
clean:
    rm -rf playwright-report/ test-results.json output/ 2>/dev/null || true
    @echo "✅ Cleaned test artifacts"

# List available tests
list-tests:
    npm run test:list | head -20

# Check if tests are passing (exit code only)
verify:
    @npm run test -- --reporter=list --max-failures=1 >/dev/null 2>&1

# Git status in concise format
status:
    git status --porcelain

# Show recent commits
log:
    git log --oneline -5