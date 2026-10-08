# Copilot premium count: research

Date 2026-10-08. Read-only research. No tokens printed.

## 1. What `gh api copilot_internal/user` returns

Run for both logged-in gh accounts (token scopes gist, read:org, repo, workflow):

- `g-akrp`: copilot_plan `individual`, access_type_sku `free_limited_copilot`. premium_interactions: entitlement 0, remaining 0, quota_remaining 0, percent_remaining 0, has_quota false, credits_used 0. Chat entitlement 200, completions 2000, both not unlimited.
- `2521180709_bblghcp`: copilot_plan `business`, access_type_sku `copilot_for_business_seat_quota`, organization_login_list empty (seat is enterprise or org managed). premium_interactions: entitlement 10000, remaining 3720, quota_remaining 3720.1, percent_remaining 37.2, overage_count 0, overage_permitted true, overage_entitlement 0, credits_used 11280, token_based_billing true, quota_reset_at 0, unlimited false, has_quota true. Chat and completions: unlimited true, entitlement 0.
- Top level on both: `token_based_billing: true`, `quota_reset_date: "2026-11-01"`, `quota_reset_date_utc: "2026-11-01T00:00:00.000Z"`.

Field meanings (observed, GitHub does not document them, so partly inferred):

- entitlement: monthly allotment. Under token-based billing this appears to be in AI credits (10000 credits = $100 at 1 credit = $0.01), no longer "requests".
- remaining: integer part of quota_remaining. quota_remaining: float, what is left. percent_remaining: quota_remaining / entitlement * 100 (3720.1 / 10000 = 37.2, matches).
- overage_count: units consumed beyond entitlement. overage_permitted: whether overage billing is allowed. overage_entitlement: only seen as 0 on the business seat; meaning unverified.
- credits_used: unverified. On the business seat it is 11280, which is larger than entitlement - remaining (6279.9) and larger than the entitlement itself while remaining is still positive, so it is not simply the month's drawdown. Possible causes (all unverified): it includes usage billed from a separate pool, includes cached or discounted tokens, or counts across a different window. Do not use it as "used".
- quota_reset_at: 0 on all snapshots, so useless. Use top-level `quota_reset_date_utc` instead.
- unlimited: true means no cap (business chat, completions); entitlement is then 0.
- Does used = entitlement - remaining hold? Yes, it agrees with percent_remaining (100 - 37.2 = 62.8 = 6279.9 / 10000). Not verified when overage is active: with overage_count > 0 I expect remaining to clamp at 0 and the excess to show in overage_count, so used = entitlement - remaining + overage_count (unverified, I have no account in overage).

## 2. Copilot Business

Business seats get the same endpoint and the same fields (observed above on a real business seat). The endpoint is not absent and does not differ. `gh auth token` tokens (OAuth app token from gh) work, no extra scope needed.

Documented billing REST API alternative:

- `GET /users/{username}/settings/billing/premium_request/usage` and `GET /organizations/{org}/settings/billing/premium_request/usage`, plus `/settings/billing/usage/summary`. Docs: https://docs.github.com/en/rest/billing/usage
- Org endpoint needs org or enterprise admin. User endpoint targets individual plans; the docs say org-managed licenses should use the org endpoint.
- Tested with the business seat token: HTTP 404 on both user endpoints, and gh says the operation needs the `user` scope. Could not confirm whether adding the scope fixes it for a business seat (unverified; likely still 404 because billing belongs to the org or enterprise).
- Returns usage items per model and SKU with quantities and amounts, not a quota remaining. It is a usage report, so it cannot give the quota alone.

Billing model context: all plans moved to usage-based billing (GitHub AI Credits, 1 credit = $0.01, token based) from June 1 2026; Business $19 per user. Completions and next edit suggestions do not consume credits. Sources: https://github.blog/news-insights/company-news/github-copilot-is-moving-to-usage-based-billing/ and https://thenewstack.io/github-copilot-usage-billing/ (search summaries only, pages not read in full). The older docs page https://docs.github.com/en/copilot/concepts/billing/copilot-requests (legacy, Pro/Pro+ only) says counters reset the 1st at 00:00 UTC.

## 3. Stability

`copilot_internal/user` is internal and undocumented. It is the endpoint Copilot clients and third-party tools use (for example https://www.mintlify.com/steipete/codexbar/providers/copilot, found via search, not verified further). The semantics already shifted: with token_based_billing the entitlement moved from request counts to credits, and new fields (credits_used, token_based_billing, overage_entitlement) appeared. Expect more drift. The only documented alternative is the billing REST API above, which needs admin or `user` scope and gives usage, not quota.

## 4. Recommendation

Keep `copilot_internal/user` as the source, it is the only one that gives quota for a business seat with the current gh token. Paths:

- quota (credits this month): `/quota_snapshots/premium_interactions/entitlement`
- left: `/quota_snapshots/premium_interactions/quota_remaining` (float; `remaining` is the integer floor)
- used: entitlement - quota_remaining (6279.9 for the business seat). Plus `/quota_snapshots/premium_interactions/overage_count` when above 0 (unverified).
- used percent: `100 - /quota_snapshots/premium_interactions/percent_remaining` (already what copilot.json does).
- reset: `/quota_reset_date_utc` (top level). Do not use `quota_reset_at`, it is 0.
- Show in credits (or dollars at credit / 100), not "requests", because token_based_billing is true. Do not use `credits_used` until its meaning is confirmed.
- Hide rule: current `entitlement != 0 and unlimited == false` works for both accounts (free account hidden, business shown). The chat and completions meters in the current mapping are correctly hidden for business (unlimited true) and show for free.

Open items: behavior during overage, meaning of credits_used, whether the `user` scope makes the billing API work for a business seat.
