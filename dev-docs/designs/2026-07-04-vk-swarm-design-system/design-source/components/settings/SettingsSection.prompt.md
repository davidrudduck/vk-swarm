Settings panel card. Wraps a `Card` with an optional leading icon in the header
and a body that vertically stacks `SettingsRow` fields with even gaps.

```jsx
<SettingsSection
  icon={<Icon d={ICONS.settings} />}
  title="Appearance"
  description="Customize how VK-Swarm looks."
>
  <SettingsRow label="Theme" htmlFor="theme" helper="Applies across the app.">
    <Select id="theme" options={themes} value={theme} onValueChange={setTheme} />
  </SettingsRow>
</SettingsSection>
```

Pass `footer` for actions (e.g. a Reset button in the card footer).
