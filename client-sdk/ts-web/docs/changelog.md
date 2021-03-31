# Changelog

## v0.1.0-alpha1

Spotlight change:

- We'll now be putting this on npm.

Breaking changes:

- The lock transaction, including the transaction body and the event, now has
  `target` field for recording a remote address.
  Similarly, the release transaction has its `owner` field renamed to
  `target`.

Note: nonbreaking changes made before v0.1.0 aren't catalogued.
Ask us directly or see the Git history for what changed.

## v0.0.2

Spotlight change:

- Method wrappers are updated to return an object that you have to sign and
  submit separately, as in `@oasisprotocol/client-rt` v0.0.2.

## v0.0.1

Spotlight change:

- After a breif, confused sequence of pull requests, this package is back, now
  with 100% less non-bridge-specific stuff in it.
