One labelled setting: label + control + helper (and optional error).

Stacked (default) — for Select / Input:
```jsx
<SettingsRow label="Git branch prefix" htmlFor="prefix"
  helper="Prepended to worktree branches." error={prefixError}>
  <Input id="prefix" value={prefix} onChange={onChange} />
</SettingsRow>
```

Inline — for a boolean Checkbox / Switch (control leads, label sits right):
```jsx
<SettingsRow inline label="Enable sound" htmlFor="sound"
  helper="Play a sound when an attempt finishes.">
  <Checkbox id="sound" checked={sound} onCheckedChange={setSound} />
</SettingsRow>
```

Add `nested` to indent a dependent row revealed under a toggle.
