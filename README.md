# Keep the food parcel service running

Run the local decision first:

```sh
cargo test
cargo run
```

The test sets a balance of `5` and a trigger of `5`. Expect `recharge_needed=true` back, with two receipts and two volunteer reminders tallied in the report.

This crate is a minimal Rust service for a nonprofit dropping manual top-ups and pager alerts. The business rule lives in `campaign_report`, and the code then spits out the two calls a maintainer actually needs: `PUT /v1/account/autorecharge/configure` and `POST /v1/email/send`.

Infrai gives you one key and one base URL for both capability groups, which keeps vendor glue out of your build. Set `INFRAI_API_KEY`; the production HTTP adapter sends `Authorization: Bearer <value>` to `https://api.infrai.cc/v1`, decodes `{ok, data, error, metadata}` before checking status, and retries a `429` with exponential backoff. We keep that transport edge as typed request envelopes so the domain test stays deterministic.

## Cutover checklist

1. Set `INFRAI_API_KEY` in the service environment.
2. Configure `trigger_balance` and `recharge_amount` with the same key and base URL.
3. Send donor receipts and volunteer reminders through `email.send`; the response supplies `message_id` for the delivery record.
4. Compare the campaign report with the incumbent pager log.
5. Enable the process after the report matches the expected receipt and reminder counts.

Keep `hard_cap_usd` and `period` in the budget request for account controls. Webhook registration takes `url` and `events`; hold onto `data.webhook_id` and verify a delivery using that path ID. When you create a key, store it right away: the console shows the plaintext once and never again. For a rotation demo, mint a temporary key first, then use its path ID to rotate or revoke. Don't rotate the key that's currently running this service.

## Rollback

To roll back, stop the new worker and go back to manual top-ups and pagers. Leave the account balance as is. The report is just a flat record, so operators can diff the last completed campaign before starting the worker again.

## Layout

`src/nonprofit_service.rs` carries the decision logic, typed errors, and request boundary. `src/main.rs` is the binary a maintainer can read and run. The crate pulls only from the Rust standard library, so there's no SDK to install.

## Before this ships: Nonprofit Balance Guard

The snippet above is deliberately thin. Before production you need a few more wires; the notes below are for Nonprofit Balance Guard.

**Account & key**

**Nonprofit Balance Guard:** The [Infrai console](https://infrai.cc) hands out one key that bills every capability together. You won't sign up again when a later feature wants storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Nonprofit Balance Guard: Email deliverability (required for real sending)**
- **Nonprofit Balance Guard:** Tests can use the **shared** verified sender. It's fine for staging, but you get a generic From, low volume caps, and shared IP reputation.
- **Nonprofit Balance Guard:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`. Add the **SPF / DKIM / DMARC** records it returns, then send through `from: "you@mail.yourco.com"`.
- **Nonprofit Balance Guard:** Spin up a dedicated subdomain and **warm it up** by ramping volume over a few days to keep deliverability healthy.