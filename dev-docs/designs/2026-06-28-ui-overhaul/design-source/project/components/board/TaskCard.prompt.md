Kanban card with a colored left strip keyed to status, plus node tag, labels, attempt indicator and a days-in-column badge.

```jsx
<TaskCard
  title="Wire up OAuth callback"
  description="Handle the redirect and persist the session token"
  status="inprogress"
  node="justX"
  labels={["auth", "backend"]}
  attempt="running"
  days={2}
/>
```

`attempt`: `running` (spinner), `merged` (green check), `failed` (coral cross).
