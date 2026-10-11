@echo off
if "%~1"=="auth" if "%~2"=="token" (
  if not "%~3"=="--user" exit /b 7
  if not "%~4"=="fixture-account" exit /b 8
  if "%AI_USAGE_TEST_AUTH_FAIL%"=="1" exit /b 1
  echo fixture-only-auth
  exit /b 0
)
if "%~1"=="api" (
  if not "%GH_TOKEN%"=="fixture-only-auth" exit /b 9
  echo {"copilot_plan":"pro","quota_reset_date_utc":"2026-11-01T00:00:00Z","quota_snapshots":{"premium_interactions":{"entitlement":300,"quota_remaining":246,"percent_remaining":82,"unlimited":false}}}
  exit /b 0
)
exit /b 10
