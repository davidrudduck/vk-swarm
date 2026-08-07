---
finding: F-2026-08-06-02
date: 2026-08-06
status: open
severity: minor (cosmetic, user-visible)
---

# Hive UI renders literal `{{when}}` placeholders — react-i18next never initialized

## Symptom
Live hive (observed 2026-08-06): node API-key rows show
`Created {{when}} · Last used {{when}}` instead of the dates.

## Root cause
`remote-frontend` components (10+, e.g. `src/components/swarm/NodeApiKeySection.tsx:92-101`)
call `useTranslation()` from `react-i18next`, but the app contains NO i18next
initialization: no `i18next.init`/`createInstance`, no `I18nextProvider`, no locale
resources anywhere under `remote-frontend/src`. An uninitialized react-i18next `t()`
returns the defaultValue string verbatim WITHOUT interpolation, so every
`{{placeholder}}` renders literally. Any `t()` call with interpolation params in
remote-frontend is affected, not just this row.

Unit tests pass because they mock the translator with an interpolating stub —
they cannot catch this class of defect.

## Recommendation
Small standalone fix: initialize a minimal i18next instance (en-only is fine) in
remote-frontend's entry point with `interpolation: { escapeValue: false }`, or drop
react-i18next in remote-frontend and use plain template strings (the hive UI is
currently en-only). Audit all `t(..., '...{{', {...})` call sites after choosing.
Consider one real-i18n smoke test (no translator mock) rendering a component with
an interpolated default to pin the regression.
