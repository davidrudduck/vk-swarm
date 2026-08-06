---
id: "201"
phase: 2
title: "Add oauth.timeoutError i18n key to all four locales"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - "frontend/src/i18n/locales/en/common.json"
  - "frontend/src/i18n/locales/ja/common.json"
  - "frontend/src/i18n/locales/ko/common.json"
  - "frontend/src/i18n/locales/es/common.json"
siblings: ["frontend/src/components/dialogs/global/OAuthDialog.tsx"]
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — covered by existing tests: none needed; pure locale data. Validity is pinned by the manual verification (JSON parse + key presence in all four files) and by task 202's component test consuming `oauth.timeoutError`.


## Change
**Files:** the four `common.json` locale files.

**Anchor:** inside the existing `"oauth"` object (en/common.json L130; ja/ko/es have the same object — locate by the `"oauth"` key). Deterministic insertion point: add the ONE new key **immediately after the existing `"tryAgain"` key** in each file (same position in all four; standard JSON comma hygiene).

- en: `"timeoutError": "Sign-in timed out. The authentication window did not complete — close it and try again."`
- ja: `"timeoutError": "サインインがタイムアウトしました。認証ウィンドウが完了しませんでした。閉じてからもう一度お試しください。"`
- ko: `"timeoutError": "로그인 시간이 초과되었습니다. 인증 창이 완료되지 않았습니다. 창을 닫고 다시 시도하세요."`
- es: `"timeoutError": "El inicio de sesión ha caducado. La ventana de autenticación no se completó; ciérrala e inténtalo de nuevo."`

Do NOT add a `retry` key — the error branch's existing `oauth.tryAgain` button is the retry affordance (it already exists in all four locales). Before editing, read sibling `OAuthDialog.tsx`'s `case 'error':` footer to confirm.


## Allowed moves
Only adding the single `timeoutError` key inside each existing `oauth` object, immediately after `tryAgain`. No other keys, files, or re-ordering of existing keys.


## STOP triggers
Any locale file lacks an `oauth` object or a `tryAgain` key; JSON parse fails after edit; a `timeoutError` key already exists; an existing `oauth` key already expresses the same timeout semantics under another name (halt and record in ledger).


## Manual verification (record in decisions-ledger)
Record in decisions-ledger: `for l in en ja ko es; do node -e "const o=require('./frontend/src/i18n/locales/$l/common.json'); if(!o.oauth.timeoutError||!o.oauth.tryAgain)process.exit(1)" && echo "$l OK"; done` — all four print OK.


## Done when
`WAI_ROOT="$(ls -d ~/.claude/plugins/cache/agent-plugins/wai/[0-9]*/ | sort -V | tail -1)"; WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="node -e \"['en','ja','ko','es'].forEach(l=>{const o=require('./frontend/src/i18n/locales/'+l+'/common.json'); if(!o.oauth.timeoutError||!o.oauth.tryAgain)process.exit(1)})\"" bash "$WAI_ROOT/scripts/task-gate.sh" hive-oauth-sw-bypass 201` exits 0
