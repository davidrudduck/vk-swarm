Themed native `<select>` with custom chevron. Good for agent / branch / config pickers.

```jsx
<Select
  defaultValue="claude"
  options={[{value:'claude',label:'Claude Code'},{value:'codex',label:'Codex'}]}
  onValueChange={setAgent}
/>
```
