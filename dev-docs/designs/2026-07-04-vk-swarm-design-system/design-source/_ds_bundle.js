/* @ds-bundle: {"format":3,"namespace":"VKSwarmDesignSystem_067861","components":[{"name":"NodeCard","sourcePath":"components/board/NodeCard.jsx"},{"name":"StatusBadge","sourcePath":"components/board/StatusBadge.jsx"},{"name":"TaskCard","sourcePath":"components/board/TaskCard.jsx"},{"name":"Badge","sourcePath":"components/core/Badge.jsx"},{"name":"Button","sourcePath":"components/core/Button.jsx"},{"name":"Card","sourcePath":"components/core/Card.jsx"},{"name":"CardHeader","sourcePath":"components/core/Card.jsx"},{"name":"CardTitle","sourcePath":"components/core/Card.jsx"},{"name":"CardDescription","sourcePath":"components/core/Card.jsx"},{"name":"CardContent","sourcePath":"components/core/Card.jsx"},{"name":"CardFooter","sourcePath":"components/core/Card.jsx"},{"name":"Checkbox","sourcePath":"components/core/Checkbox.jsx"},{"name":"Input","sourcePath":"components/core/Input.jsx"},{"name":"Loader","sourcePath":"components/core/Loader.jsx"},{"name":"Select","sourcePath":"components/core/Select.jsx"},{"name":"Switch","sourcePath":"components/core/Switch.jsx"},{"name":"Tabs","sourcePath":"components/core/Tabs.jsx"},{"name":"SettingsRow","sourcePath":"components/settings/SettingsRow.jsx"},{"name":"SettingsSection","sourcePath":"components/settings/SettingsSection.jsx"}],"sourceHashes":{"components/board/NodeCard.jsx":"6285993edede","components/board/StatusBadge.jsx":"d672f7013791","components/board/TaskCard.jsx":"097f6ba1ca13","components/core/Badge.jsx":"dfbb82574b10","components/core/Button.jsx":"361480177954","components/core/Card.jsx":"5b720b78c61b","components/core/Checkbox.jsx":"12445b91810b","components/core/Input.jsx":"c41df2a3fe50","components/core/Loader.jsx":"3d22713dcbb5","components/core/Select.jsx":"e022b2052d92","components/core/Switch.jsx":"54da83b1dfb8","components/core/Tabs.jsx":"b815cb74ab0b","components/settings/SettingsRow.jsx":"e8a4af6fa575","components/settings/SettingsSection.jsx":"2e538ffbba8f","ui_kits/vk-swarm-app/board.jsx":"075566a2f679","ui_kits/vk-swarm-app/chrome.jsx":"33b35379e343","ui_kits/vk-swarm-app/panels.jsx":"59ec16015f62","ui_kits/vk-swarm-app/settings-panels.jsx":"575be3ef1b3e","ui_kits/vk-swarm-app/settings.jsx":"6c3d0ff84cb8"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.VKSwarmDesignSystem_067861 = window.VKSwarmDesignSystem_067861 || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/board/NodeCard.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const OS_GLYPH = {
  mac: /*#__PURE__*/React.createElement("path", {
    d: "M12 4.5c.4-1 .3-2 .2-2.3-.9.1-1.9.7-2.5 1.4-.5.6-.9 1.6-.8 2.5 1 .1 2-.5 2.6-1.2.3-.1.4-.3.5-.4zM14.8 11.4c-.5 1.1-.7 1.6-1.4 2.6-.9 1.3-2.2 3-3.8 3-1.4 0-1.8-.9-3.7-.9s-2.3.9-3.7.9c-1.6 0-2.8-1.5-3.7-2.8C-.8 12.1-.3 9 1 7.2c.9-1.3 2.2-2 3.5-2 1.4 0 2.2.9 3.4.9 1.1 0 1.8-.9 3.4-.9 1.2 0 2.5.6 3.4 1.7-3 1.6-2.5 5.9-.3 4.4z",
    transform: "translate(1 0)"
  }),
  linux: /*#__PURE__*/React.createElement("path", {
    d: "M8 1c-1.8 0-2.6 1.6-2.6 3.4 0 1 .2 1.7.2 2.6 0 1-.9 1.8-1.6 3-.7 1.2-1.4 2.4-1.4 3.6 0 .9.5 1.4 1.3 1.4.6 0 1-.3 1.4-.3.3 0 .5.2.8.4.5.3 1.2.5 2 .5s1.5-.2 2-.5c.3-.2.5-.4.8-.4.4 0 .8.3 1.4.3.8 0 1.3-.5 1.3-1.4 0-1.2-.7-2.4-1.4-3.6-.7-1.2-1.6-2-1.6-3 0-.9.2-1.6.2-2.6C10.6 2.6 9.8 1 8 1z"
  }),
  windows: /*#__PURE__*/React.createElement("path", {
    d: "M1 2.8l5.7-.8v5.5H1V2.8zm6.4-.9L15 1v6.5H7.4V1.9zM1 8.2h5.7v5.5L1 13V8.2zm6.4 0H15V15l-7.6-1V8.2z"
  })
};

/** Swarm node row: OS glyph, name, status pulse, optional meta. */
function NodeCard({
  name,
  os = 'linux',
  online = true,
  meta,
  right,
  className = '',
  ...props
}) {
  const cls = ['vks-node', className].filter(Boolean).join(' ');
  return /*#__PURE__*/React.createElement("div", _extends({
    className: cls
  }, props), /*#__PURE__*/React.createElement("div", {
    className: "vks-node__os"
  }, /*#__PURE__*/React.createElement("svg", {
    width: "18",
    height: "18",
    viewBox: "0 0 16 16",
    fill: "currentColor",
    "aria-hidden": "true"
  }, OS_GLYPH[os] || OS_GLYPH.linux)), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0,
      flex: 1
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    className: "vks-node__name"
  }, name), /*#__PURE__*/React.createElement("span", {
    className: online ? 'vks-node__pulse' : 'vks-node__pulse vks-node__pulse--offline'
  })), meta && /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 'var(--text-sm)',
      color: 'var(--text-muted)',
      marginTop: 2
    }
  }, meta)), right);
}
Object.assign(__ds_scope, { NodeCard });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/board/NodeCard.jsx", error: String((e && e.message) || e) }); }

// components/board/StatusBadge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const LABELS = {
  todo: 'To Do',
  inprogress: 'In Progress',
  inreview: 'In Review',
  done: 'Done',
  cancelled: 'Cancelled'
};

/** Status indicator (dot + label) matching the kanban column colors. */
function StatusBadge({
  status = 'todo',
  showLabel = true,
  label,
  className = '',
  ...props
}) {
  const cls = ['vks-status', `vks-status--${status}`, className].filter(Boolean).join(' ');
  return /*#__PURE__*/React.createElement("span", _extends({
    className: cls
  }, props), /*#__PURE__*/React.createElement("span", {
    className: "vks-status__dot"
  }), showLabel && /*#__PURE__*/React.createElement("span", null, label ?? LABELS[status] ?? status));
}
Object.assign(__ds_scope, { StatusBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/board/StatusBadge.jsx", error: String((e && e.message) || e) }); }

// components/board/TaskCard.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Kanban task card with left status strip. Composes title, description, node
 * tag, labels and an attempt indicator.
 */
function TaskCard({
  title,
  description,
  status = 'todo',
  node,
  labels = [],
  attempt,
  // 'running' | 'merged' | 'failed' | undefined
  days,
  className = '',
  ...props
}) {
  const cls = ['vks-task', `vks-task--${status}`, className].filter(Boolean).join(' ');
  return /*#__PURE__*/React.createElement("div", _extends({
    className: cls
  }, props), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'space-between',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("p", {
    className: "vks-task__title"
  }, title), attempt && /*#__PURE__*/React.createElement(AttemptIndicator, {
    attempt: attempt
  })), description && /*#__PURE__*/React.createElement("p", {
    className: "vks-task__desc",
    title: description
  }, description), /*#__PURE__*/React.createElement("div", {
    className: "vks-task__meta"
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      minWidth: 0
    }
  }, node && /*#__PURE__*/React.createElement("span", {
    className: "vks-task__node"
  }, node), labels.slice(0, 2).map(l => /*#__PURE__*/React.createElement("span", {
    key: l,
    className: "vks-badge vks-badge--outline",
    style: {
      padding: '1px 7px',
      fontSize: 'var(--text-xs)'
    }
  }, l))), days != null && /*#__PURE__*/React.createElement("span", {
    className: "vks-badge vks-badge--secondary",
    style: {
      padding: '1px 7px',
      fontSize: 'var(--text-xs)'
    },
    title: "Days in column"
  }, days, "d")));
}
function AttemptIndicator({
  attempt
}) {
  if (attempt === 'running') return /*#__PURE__*/React.createElement("span", {
    className: "vks-loader",
    style: {
      width: 14,
      height: 14,
      flexShrink: 0
    },
    "aria-label": "Running"
  });
  const color = attempt === 'merged' ? 'var(--success)' : 'var(--danger)';
  const path = attempt === 'merged' ? /*#__PURE__*/React.createElement("path", {
    d: "M5 8.5l2 2 4-4.5",
    stroke: color,
    strokeWidth: "1.6",
    strokeLinecap: "round",
    strokeLinejoin: "round",
    fill: "none"
  }) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M6 6l4 4M10 6l-4 4",
    stroke: color,
    strokeWidth: "1.6",
    strokeLinecap: "round"
  }));
  return /*#__PURE__*/React.createElement("svg", {
    width: "16",
    height: "16",
    viewBox: "0 0 16 16",
    style: {
      flexShrink: 0
    },
    "aria-label": attempt
  }, /*#__PURE__*/React.createElement("circle", {
    cx: "8",
    cy: "8",
    r: "7",
    stroke: color,
    strokeWidth: "1.3",
    fill: "none",
    opacity: "0.5"
  }), path);
}
Object.assign(__ds_scope, { TaskCard });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/board/TaskCard.jsx", error: String((e && e.message) || e) }); }

// components/core/Badge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const VARIANTS = {
  default: 'vks-badge--default',
  secondary: 'vks-badge--secondary',
  destructive: 'vks-badge--destructive',
  outline: 'vks-badge--outline'
};

/** Small rounded-full label. Optional leading dot for counts / statuses. */
function Badge({
  variant = 'default',
  dot = false,
  className = '',
  children,
  ...props
}) {
  const cls = ['vks-badge', VARIANTS[variant] || VARIANTS.default, className].filter(Boolean).join(' ');
  return /*#__PURE__*/React.createElement("span", _extends({
    className: cls
  }, props), dot && /*#__PURE__*/React.createElement("span", {
    className: "vks-badge__dot"
  }), children);
}
Object.assign(__ds_scope, { Badge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Badge.jsx", error: String((e && e.message) || e) }); }

// components/core/Button.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  xs: 'vks-btn--xs',
  sm: 'vks-btn--sm',
  md: 'vks-btn--md',
  lg: 'vks-btn--lg',
  icon: 'vks-btn--icon'
};
const VARIANTS = {
  primary: 'vks-btn--primary',
  secondary: 'vks-btn--secondary',
  outline: 'vks-btn--outline',
  ghost: 'vks-btn--ghost',
  destructive: 'vks-btn--destructive',
  link: 'vks-btn--link'
};

/**
 * VK-Swarm button. Mirrors the app's cva variants (default→primary, outline,
 * ghost, destructive, link) with compact terminal-dense sizing.
 */
function Button({
  variant = 'primary',
  size = 'md',
  className = '',
  children,
  ...props
}) {
  const cls = ['vks-btn', VARIANTS[variant] || VARIANTS.primary, SIZES[size] || SIZES.md, className].filter(Boolean).join(' ');
  return /*#__PURE__*/React.createElement("button", _extends({
    className: cls
  }, props), children);
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Button.jsx", error: String((e && e.message) || e) }); }

// components/core/Card.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
function Card({
  className = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("div", _extends({
    className: ['vks-card', className].filter(Boolean).join(' ')
  }, props), children);
}
function CardHeader({
  className = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("div", _extends({
    className: ['vks-card__header', className].filter(Boolean).join(' ')
  }, props), children);
}
function CardTitle({
  className = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("h3", _extends({
    className: ['vks-card__title', className].filter(Boolean).join(' ')
  }, props), children);
}
function CardDescription({
  className = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("p", _extends({
    className: ['vks-card__desc', className].filter(Boolean).join(' ')
  }, props), children);
}
function CardContent({
  className = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("div", _extends({
    className: ['vks-card__content', className].filter(Boolean).join(' ')
  }, props), children);
}
function CardFooter({
  className = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("div", _extends({
    className: ['vks-card__footer', className].filter(Boolean).join(' ')
  }, props), children);
}
Object.assign(__ds_scope, { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Card.jsx", error: String((e && e.message) || e) }); }

// components/core/Checkbox.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** Controlled or uncontrolled checkbox. */
function Checkbox({
  checked,
  defaultChecked = false,
  onCheckedChange,
  disabled = false,
  className = '',
  ...props
}) {
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(defaultChecked);
  const on = isControlled ? checked : internal;
  const toggle = () => {
    if (disabled) return;
    if (!isControlled) setInternal(!on);
    onCheckedChange && onCheckedChange(!on);
  };
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    role: "checkbox",
    "aria-checked": on,
    "data-checked": on,
    disabled: disabled,
    onClick: toggle,
    className: ['vks-checkbox', className].filter(Boolean).join(' ')
  }, props), /*#__PURE__*/React.createElement("svg", {
    width: "11",
    height: "11",
    viewBox: "0 0 12 12",
    fill: "none",
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M2.5 6.5l2.5 2.5 4.5-5",
    stroke: "currentColor",
    strokeWidth: "2",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  })));
}
Object.assign(__ds_scope, { Checkbox });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Checkbox.jsx", error: String((e && e.message) || e) }); }

// components/core/Input.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** Text input on `--input` surface. Pass `mono` for code/branch fields. */
function Input({
  mono = false,
  className = '',
  ...props
}) {
  const cls = ['vks-input', mono && 'vks-input--mono', className].filter(Boolean).join(' ');
  return /*#__PURE__*/React.createElement("input", _extends({
    className: cls
  }, props));
}
Object.assign(__ds_scope, { Input });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Input.jsx", error: String((e && e.message) || e) }); }

// components/core/Loader.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const SIZES = {
  sm: 14,
  md: 18,
  lg: 28
};

/** Spinner. `size` is sm|md|lg or a pixel number. */
function Loader({
  size = 'md',
  className = '',
  style,
  ...props
}) {
  const px = typeof size === 'number' ? size : SIZES[size] || SIZES.md;
  return /*#__PURE__*/React.createElement("span", _extends({
    className: ['vks-loader', className].filter(Boolean).join(' '),
    style: {
      width: px,
      height: px,
      ...style
    },
    role: "status",
    "aria-label": "Loading"
  }, props));
}
Object.assign(__ds_scope, { Loader });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Loader.jsx", error: String((e && e.message) || e) }); }

// components/core/Select.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** Native select styled for the Midnight Terminal theme. */
function Select({
  options = [],
  value,
  defaultValue,
  onValueChange,
  disabled = false,
  className = '',
  ...props
}) {
  const isControlled = value !== undefined;
  const [internal, setInternal] = React.useState(defaultValue ?? (options[0] && options[0].value));
  const v = isControlled ? value : internal;
  const change = e => {
    if (!isControlled) setInternal(e.target.value);
    onValueChange && onValueChange(e.target.value);
  };
  return /*#__PURE__*/React.createElement("div", {
    className: ['vks-select', className].filter(Boolean).join(' ')
  }, /*#__PURE__*/React.createElement("select", _extends({
    value: v,
    onChange: change,
    disabled: disabled
  }, props), options.map(o => /*#__PURE__*/React.createElement("option", {
    key: o.value,
    value: o.value
  }, o.label))), /*#__PURE__*/React.createElement("span", {
    className: "vks-select__chevron",
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "12",
    viewBox: "0 0 12 12",
    fill: "none"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M3 4.5L6 7.5L9 4.5",
    stroke: "currentColor",
    strokeWidth: "1.5",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }))));
}
Object.assign(__ds_scope, { Select });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Select.jsx", error: String((e && e.message) || e) }); }

// components/core/Switch.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** Controlled or uncontrolled toggle switch. */
function Switch({
  checked,
  defaultChecked = false,
  onCheckedChange,
  disabled = false,
  className = '',
  ...props
}) {
  const isControlled = checked !== undefined;
  const [internal, setInternal] = React.useState(defaultChecked);
  const on = isControlled ? checked : internal;
  const toggle = () => {
    if (disabled) return;
    if (!isControlled) setInternal(!on);
    onCheckedChange && onCheckedChange(!on);
  };
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    role: "switch",
    "aria-checked": on,
    "data-checked": on,
    disabled: disabled,
    onClick: toggle,
    className: ['vks-switch', className].filter(Boolean).join(' ')
  }, props), /*#__PURE__*/React.createElement("span", {
    className: "vks-switch__thumb"
  }));
}
Object.assign(__ds_scope, { Switch });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Switch.jsx", error: String((e && e.message) || e) }); }

// components/core/Tabs.jsx
try { (() => {
/**
 * Segmented tab control.
 * @param {{value:string,label:React.ReactNode}[]} tabs
 */
function Tabs({
  tabs = [],
  value,
  defaultValue,
  onValueChange,
  className = ''
}) {
  const isControlled = value !== undefined;
  const [internal, setInternal] = React.useState(defaultValue ?? (tabs[0] && tabs[0].value));
  const active = isControlled ? value : internal;
  const select = v => {
    if (!isControlled) setInternal(v);
    onValueChange && onValueChange(v);
  };
  return /*#__PURE__*/React.createElement("div", {
    className: ['vks-tabs__list', className].filter(Boolean).join(' '),
    role: "tablist"
  }, tabs.map(t => /*#__PURE__*/React.createElement("button", {
    key: t.value,
    role: "tab",
    "aria-selected": active === t.value,
    "data-active": active === t.value,
    className: "vks-tabs__trigger",
    onClick: () => select(t.value)
  }, t.label)));
}
Object.assign(__ds_scope, { Tabs });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Tabs.jsx", error: String((e && e.message) || e) }); }

// components/settings/SettingsRow.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * A single labelled setting: a label, its control, and helper/error text.
 *
 * - Default (stacked): label on top, control, then helper below — used for
 *   Select / Input rows.
 * - `inline`: a leading control (Checkbox / Switch) followed by a label +
 *   helper stack to its right — used for boolean toggles.
 *
 * The control is passed as `control` or as children.
 */
function SettingsRow({
  label,
  htmlFor,
  helper,
  error,
  inline = false,
  nested = false,
  control,
  className = '',
  children,
  ...props
}) {
  const body = control ?? children;
  const cls = ['vks-field', inline && 'vks-field--inline', nested && 'vks-field--nested', className].filter(Boolean).join(' ');
  if (inline) {
    return /*#__PURE__*/React.createElement("div", _extends({
      className: cls
    }, props), body, /*#__PURE__*/React.createElement("div", {
      className: "vks-field__body"
    }, label && /*#__PURE__*/React.createElement("label", {
      htmlFor: htmlFor,
      className: "vks-field__label"
    }, label), helper && /*#__PURE__*/React.createElement("p", {
      className: "vks-field__helper"
    }, helper), error && /*#__PURE__*/React.createElement("p", {
      className: "vks-field__error"
    }, error)));
  }
  return /*#__PURE__*/React.createElement("div", _extends({
    className: cls
  }, props), label && /*#__PURE__*/React.createElement("label", {
    htmlFor: htmlFor,
    className: "vks-field__label"
  }, label), body, error && /*#__PURE__*/React.createElement("p", {
    className: "vks-field__error"
  }, error), helper && !error && /*#__PURE__*/React.createElement("p", {
    className: "vks-field__helper"
  }, helper));
}
Object.assign(__ds_scope, { SettingsRow });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/settings/SettingsRow.jsx", error: String((e && e.message) || e) }); }

// components/settings/SettingsSection.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * A settings panel: a Card with a header (title, optional description and
 * leading icon) whose body vertically stacks its child fields with even gaps.
 * Pass a `footer` for actions like Reset. Mirrors the app's Card + CardHeader
 * pattern used across every settings page.
 */
function SettingsSection({
  title,
  description,
  icon,
  footer,
  className = '',
  contentClassName = '',
  children,
  ...props
}) {
  return /*#__PURE__*/React.createElement("section", _extends({
    className: ['vks-card', className].filter(Boolean).join(' ')
  }, props), /*#__PURE__*/React.createElement("div", {
    className: ['vks-card__header', icon && 'vks-settings__header'].filter(Boolean).join(' ')
  }, icon && /*#__PURE__*/React.createElement("span", {
    className: "vks-settings__header-icon"
  }, icon), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("h3", {
    className: "vks-card__title"
  }, title), description && /*#__PURE__*/React.createElement("p", {
    className: "vks-card__desc"
  }, description))), /*#__PURE__*/React.createElement("div", {
    className: ['vks-card__content', 'vks-settings__body', contentClassName].filter(Boolean).join(' ')
  }, children), footer && /*#__PURE__*/React.createElement("div", {
    className: "vks-card__footer"
  }, footer));
}
Object.assign(__ds_scope, { SettingsSection });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/settings/SettingsSection.jsx", error: String((e && e.message) || e) }); }

// ui_kits/vk-swarm-app/board.jsx
try { (() => {
// VK-Swarm UI kit — Kanban board view. Uses TaskCard from the bundle.
const {
  useState
} = window.React;
const SEED = {
  todo: [{
    id: 't1',
    title: 'Add rate limiting to hive WebSocket',
    description: 'Throttle node reconnect storms during deploys',
    node: 'justX',
    labels: ['infra'],
    days: 1
  }, {
    id: 't2',
    title: 'Document swarm-hive setup',
    description: 'Walk through VK_HIVE_URL and node API keys',
    node: 'linux-01',
    labels: ['docs'],
    days: 3
  }],
  inprogress: [{
    id: 't3',
    title: 'Wire up OAuth callback',
    description: 'Handle redirect and persist the session token',
    node: 'justX',
    labels: ['auth', 'backend'],
    attempt: 'running',
    days: 2
  }, {
    id: 't4',
    title: 'Diff view virtualization',
    description: 'Render large diffs without jank',
    node: 'winbox',
    labels: ['ui'],
    attempt: 'running',
    days: 1
  }],
  inreview: [{
    id: 't5',
    title: 'Migrate hive schema to pgvector',
    description: 'Embedding columns + backfill job',
    node: 'linux-01',
    labels: ['db'],
    attempt: 'failed',
    days: 4
  }],
  done: [{
    id: 't6',
    title: 'Add DiffViewSwitch component',
    node: 'justX',
    labels: ['ui'],
    attempt: 'merged',
    days: 6
  }, {
    id: 't7',
    title: 'Compact label list on cards',
    node: 'winbox',
    labels: ['ui'],
    attempt: 'merged',
    days: 8
  }],
  cancelled: [{
    id: 't8',
    title: 'Experiment: local SQLite-only mode',
    node: 'justX',
    days: 12
  }]
};
const COLUMNS = [{
  key: 'todo',
  label: 'To Do',
  color: 'var(--status-todo)'
}, {
  key: 'inprogress',
  label: 'In Progress',
  color: 'var(--status-inprogress)'
}, {
  key: 'inreview',
  label: 'In Review',
  color: 'var(--status-inreview)'
}, {
  key: 'done',
  label: 'Done',
  color: 'var(--status-done)'
}, {
  key: 'cancelled',
  label: 'Cancelled',
  color: 'var(--status-cancelled)'
}];
function ColumnHeader({
  col,
  count,
  onAdd
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      top: 0,
      zIndex: 2,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '10px 12px',
      background: 'var(--background)',
      borderBottom: '1px dashed var(--border)',
      backgroundImage: `linear-gradient(color-mix(in srgb, ${col.color} 8%, transparent), transparent)`
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 9,
      height: 9,
      borderRadius: '50%',
      background: col.color,
      flexShrink: 0
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 'var(--text-sm)',
      fontWeight: 600
    }
  }, col.label), /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)',
      background: 'var(--surface-card)',
      padding: '1px 7px',
      borderRadius: 4,
      fontVariantNumeric: 'tabular-nums'
    }
  }, count), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost",
    style: {
      height: 24,
      width: 24,
      padding: 0
    },
    onClick: onAdd,
    title: "Add task"
  }, /*#__PURE__*/React.createElement(window.Icon, {
    d: window.ICONS.plus,
    size: 14
  })));
}
function BoardView({
  columns,
  onAdd,
  onOpen,
  selectedId
}) {
  const {
    TaskCard
  } = window.VKSwarmDesignSystem_067861;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridAutoFlow: 'column',
      gridAutoColumns: 'minmax(264px, 1fr)',
      height: '100%',
      overflowX: 'auto',
      borderLeft: '1px solid var(--border)'
    }
  }, COLUMNS.map(col => /*#__PURE__*/React.createElement("div", {
    key: col.key,
    style: {
      display: 'flex',
      flexDirection: 'column',
      borderRight: '1px solid var(--border)',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement(ColumnHeader, {
    col: col,
    count: columns[col.key].length,
    onAdd: () => onAdd(col.key)
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8,
      padding: 10,
      overflowY: 'auto',
      flex: 1
    }
  }, columns[col.key].map(t => /*#__PURE__*/React.createElement(TaskCard, {
    key: t.id,
    title: t.title,
    description: t.description,
    status: col.key,
    node: t.node,
    labels: t.labels,
    attempt: t.attempt,
    days: t.days,
    onClick: () => onOpen(t, col.key),
    style: selectedId === t.id ? {
      boxShadow: '0 0 0 2px var(--primary)',
      borderColor: 'var(--primary)'
    } : null
  })), columns[col.key].length === 0 && /*#__PURE__*/React.createElement("div", {
    className: "vks-ansi-dither vks-scanlines",
    style: {
      borderRadius: 'var(--radius-md)',
      border: '1px solid var(--border)',
      minHeight: 80,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: 'var(--text-muted)',
      fontSize: 'var(--text-xs)',
      fontFamily: 'var(--font-code)',
      letterSpacing: '0.06em'
    }
  }, "\u2591\u2592 no tasks \u2592\u2591")))));
}
Object.assign(window, {
  BoardView,
  SEED,
  COLUMNS
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/vk-swarm-app/board.jsx", error: String((e && e.message) || e) }); }

// ui_kits/vk-swarm-app/chrome.jsx
try { (() => {
// VK-Swarm UI kit — shared chrome (navbar) and small primitives built on the
// design-system bundle classes. Components register on window for sibling files.
const {
  useState
} = React;
const Icon = ({
  d,
  size = 16,
  stroke = 1.6,
  fill = 'none'
}) => /*#__PURE__*/React.createElement("svg", {
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: fill,
  stroke: "currentColor",
  strokeWidth: stroke,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": "true"
}, d);

// Lucide-style stroke icons (24px grid, ~1.6px) — the product uses lucide-react.
const ICONS = {
  plus: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M12 5v14M5 12h14"
  })),
  search: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "11",
    cy: "11",
    r: "7"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M21 21l-4.3-4.3"
  })),
  server: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "4",
    width: "18",
    height: "7",
    rx: "1.5"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "3",
    y: "13",
    width: "18",
    height: "7",
    rx: "1.5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M7 7.5h.01M7 16.5h.01"
  })),
  folder: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
  })),
  activity: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M22 12h-4l-3 9L9 3l-3 9H2"
  })),
  settings: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-2.82 1.17V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 8 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 14H4.5a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 6 8.6a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 10 4.6h.09A1.65 1.65 0 0 0 11.4 3h.09a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 16 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 8z"
  })),
  menu: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M4 6h16M4 12h16M4 18h16"
  })),
  git: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "6",
    cy: "6",
    r: "2.5"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "6",
    cy: "18",
    r: "2.5"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "18",
    cy: "9",
    r: "2.5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M6 8.5v7M18 11.5c0 4-6 1.5-6 4.5"
  })),
  bolt: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M13 2L4.5 13H11l-1 9 8.5-11H12z"
  })),
  sun: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "12",
    cy: "12",
    r: "4"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"
  })),
  moon: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"
  }))
};

// Track viewport width so chrome adapts across mobile / tablet / desktop.
function useBreakpoint() {
  const get = () => typeof window === 'undefined' ? 'desktop' : window.innerWidth < 640 ? 'mobile' : window.innerWidth < 1024 ? 'tablet' : 'desktop';
  const [bp, setBp] = useState(get);
  React.useEffect(() => {
    const on = () => setBp(get());
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return bp;
}
function Logo({
  compact
}) {
  return /*#__PURE__*/React.createElement("span", {
    className: "vks-wordmark",
    style: {
      fontSize: compact ? 16 : 18
    }
  }, /*#__PURE__*/React.createElement("span", {
    className: "vk"
  }, "VK"), /*#__PURE__*/React.createElement("span", {
    className: "swarm"
  }, compact ? 'S' : '-SWARM'));
}
function ThemeToggle({
  theme,
  onToggle
}) {
  return /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost vks-btn--icon",
    onClick: onToggle,
    title: theme === 'dark' ? 'Switch to light' : 'Switch to dark',
    "aria-label": "Toggle theme",
    style: {
      height: 34,
      width: 34
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: theme === 'dark' ? ICONS.sun : ICONS.moon,
    size: 16
  }));
}
function Navbar({
  project,
  view,
  onView,
  onNewTask,
  theme,
  onToggleTheme,
  onOpenSettings
}) {
  const bp = useBreakpoint();
  const mobile = bp === 'mobile';
  const tablet = bp === 'tablet';
  return /*#__PURE__*/React.createElement("header", {
    style: {
      borderBottom: '1px solid var(--border)',
      background: 'var(--background)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      height: 48,
      padding: '0 12px',
      gap: mobile ? 8 : 12
    }
  }, /*#__PURE__*/React.createElement(Logo, {
    compact: mobile
  }), !mobile && /*#__PURE__*/React.createElement("div", {
    style: {
      width: 1,
      height: 22,
      background: 'var(--border)',
      margin: '0 2px'
    }
  }), /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost vks-btn--sm",
    style: {
      gap: 8,
      paddingLeft: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: ICONS.folder,
    size: 14
  })), !mobile && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 'var(--text-sm)',
      fontWeight: 600
    }
  }, project), /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-dim)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: /*#__PURE__*/React.createElement("path", {
      d: "M6 9l6 6 6-6"
    }),
    size: 12
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }), bp === 'desktop' ? /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: 260,
      maxWidth: '30vw'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: 10,
      top: '50%',
      transform: 'translateY(-50%)',
      color: 'var(--text-dim)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: ICONS.search,
    size: 14
  })), /*#__PURE__*/React.createElement("input", {
    className: "vks-input",
    placeholder: "Search tasks\u2026",
    style: {
      height: 34,
      paddingLeft: 32,
      fontSize: 'var(--text-sm)'
    }
  })) : /*#__PURE__*/React.createElement(NavIcon, {
    icon: ICONS.search,
    title: "Search"
  }), !mobile && /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost vks-btn--icon",
    title: "Open in IDE",
    style: {
      height: 34,
      width: 34
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: ICONS.bolt,
    size: 16
  })), /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--primary vks-btn--sm",
    onClick: onNewTask,
    style: {
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: ICONS.plus,
    size: 14
  }), " ", !mobile && 'Task'), !mobile && /*#__PURE__*/React.createElement("div", {
    style: {
      width: 1,
      height: 22,
      background: 'var(--border)',
      margin: '0 2px'
    }
  }), /*#__PURE__*/React.createElement(ThemeToggle, {
    theme: theme,
    onToggle: onToggleTheme
  }), !tablet && !mobile && /*#__PURE__*/React.createElement(NavIcon, {
    icon: ICONS.activity,
    title: "Activity"
  }), !mobile && /*#__PURE__*/React.createElement(NavIcon, {
    icon: ICONS.settings,
    title: "Settings",
    onClick: onOpenSettings
  }), /*#__PURE__*/React.createElement(NavIcon, {
    icon: ICONS.menu,
    title: "Menu"
  })), /*#__PURE__*/React.createElement("nav", {
    style: {
      display: 'flex',
      gap: 2,
      padding: '0 12px',
      overflowX: 'auto'
    }
  }, /*#__PURE__*/React.createElement(NavTab, {
    active: view === 'board',
    onClick: () => onView('board'),
    icon: ICONS.folder,
    label: "Board"
  }), /*#__PURE__*/React.createElement(NavTab, {
    active: view === 'nodes',
    onClick: () => onView('nodes'),
    icon: ICONS.server,
    label: "Nodes"
  }), /*#__PURE__*/React.createElement(NavTab, {
    active: view === 'processes',
    onClick: () => onView('processes'),
    icon: ICONS.activity,
    label: "Processes"
  })));
}
function NavIcon({
  icon,
  title,
  onClick
}) {
  return /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost vks-btn--icon",
    title: title,
    onClick: onClick,
    style: {
      height: 34,
      width: 34
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: icon,
    size: 16
  }));
}
function NavTab({
  active,
  onClick,
  icon,
  label
}) {
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      padding: '9px 12px',
      background: 'transparent',
      border: 0,
      borderBottom: `2px solid ${active ? 'var(--primary)' : 'transparent'}`,
      color: active ? 'var(--foreground)' : 'var(--text-muted)',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--text-sm)',
      fontWeight: 500,
      cursor: 'pointer',
      marginBottom: -1,
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    d: icon,
    size: 14
  }), " ", label);
}
Object.assign(window, {
  Icon,
  ICONS,
  Navbar,
  Logo,
  useBreakpoint,
  ThemeToggle
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/vk-swarm-app/chrome.jsx", error: String((e && e.message) || e) }); }

// ui_kits/vk-swarm-app/panels.jsx
try { (() => {
// VK-Swarm UI kit — Nodes view + Task detail drawer + Processes placeholder.
const {
  useState
} = window.React;
function NodesView() {
  const {
    NodeCard,
    Badge,
    Button
  } = window.VKSwarmDesignSystem_067861;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      padding: 20,
      overflowY: 'auto',
      height: '100%'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginBottom: 16
    }
  }, /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'var(--text-2xl)',
      fontWeight: 600,
      margin: 0
    }
  }, "Hive"), /*#__PURE__*/React.createElement("span", {
    className: "vks-status vks-status--done"
  }, /*#__PURE__*/React.createElement("span", {
    className: "vks-status__dot"
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 'var(--text-sm)'
    }
  }, "3 nodes online"))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))',
      gap: 12,
      maxWidth: 1000
    }
  }, /*#__PURE__*/React.createElement(NodeCard, {
    name: "justX.raverx.net",
    os: "mac",
    online: true,
    meta: "3 agents \xB7 wss://hive.raverx.net",
    right: /*#__PURE__*/React.createElement(Badge, {
      variant: "secondary",
      dot: true
    }, "3")
  }), /*#__PURE__*/React.createElement(NodeCard, {
    name: "linux-01",
    os: "linux",
    online: true,
    meta: "1 agent \xB7 streaming logs",
    right: /*#__PURE__*/React.createElement(Badge, {
      variant: "secondary",
      dot: true
    }, "1")
  }), /*#__PURE__*/React.createElement(NodeCard, {
    name: "winbox",
    os: "windows",
    online: true,
    meta: "2 agents \xB7 direct connect",
    right: /*#__PURE__*/React.createElement(Badge, {
      variant: "secondary",
      dot: true
    }, "2")
  }), /*#__PURE__*/React.createElement(NodeCard, {
    name: "ci-runner-04",
    os: "linux",
    online: false,
    meta: "last seen 4m ago",
    right: /*#__PURE__*/React.createElement(Badge, {
      variant: "outline"
    }, "offline")
  })));
}
function ProcessesView() {
  const rows = [{
    name: 'claude-code · feat/auth',
    node: 'justX',
    state: 'running',
    dur: '2m 14s'
  }, {
    name: 'dev-server · vite',
    node: 'justX',
    state: 'running',
    dur: '41m'
  }, {
    name: 'codex · diff-virtualization',
    node: 'winbox',
    state: 'running',
    dur: '58s'
  }, {
    name: 'pnpm test',
    node: 'linux-01',
    state: 'done',
    dur: '1m 02s'
  }];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      padding: 20,
      overflowY: 'auto',
      height: '100%'
    }
  }, /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'var(--text-2xl)',
      fontWeight: 600,
      margin: '0 0 14px'
    }
  }, "Processes"), /*#__PURE__*/React.createElement("div", {
    className: "vks-card",
    style: {
      overflow: 'hidden',
      maxWidth: 860
    }
  }, rows.map((r, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '12px 16px',
      borderBottom: i < rows.length - 1 ? '1px solid var(--border)' : 0
    }
  }, r.state === 'running' ? /*#__PURE__*/React.createElement("span", {
    className: "vks-loader",
    style: {
      width: 14,
      height: 14
    }
  }) : /*#__PURE__*/React.createElement("span", {
    className: "vks-status vks-status--done"
  }, /*#__PURE__*/React.createElement("span", {
    className: "vks-status__dot"
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)',
      flex: 1
    }
  }, r.name), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)'
    }
  }, r.node), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-dim)',
      width: 56,
      textAlign: 'right'
    }
  }, r.dur)))));
}
function TaskDrawer({
  task,
  status,
  onClose
}) {
  const {
    Button,
    Badge,
    StatusBadge,
    Tabs
  } = window.VKSwarmDesignSystem_067861;
  const [tab, setTab] = useState('diff');
  if (!task) return null;
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'absolute',
      inset: 0,
      background: 'var(--surface-overlay)',
      zIndex: 10
    }
  }), /*#__PURE__*/React.createElement("aside", {
    style: {
      position: 'absolute',
      top: 0,
      right: 0,
      bottom: 0,
      width: 460,
      maxWidth: '90%',
      zIndex: 11,
      background: 'var(--surface-card)',
      borderLeft: '1px solid var(--border-strong)',
      boxShadow: 'var(--shadow-lg)',
      display: 'flex',
      flexDirection: 'column'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px 18px',
      borderBottom: '1px solid var(--border)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-start',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(StatusBadge, {
    status: status,
    showLabel: false
  }), /*#__PURE__*/React.createElement("h3", {
    style: {
      fontSize: 'var(--text-lg)',
      fontWeight: 600,
      margin: 0,
      flex: 1,
      lineHeight: 1.3
    }
  }, task.title), /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost vks-btn--icon",
    onClick: onClose,
    style: {
      height: 28,
      width: 28
    }
  }, /*#__PURE__*/React.createElement(window.Icon, {
    d: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
      d: "M6 6l12 12M18 6L6 18"
    })),
    size: 16
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6,
      marginTop: 12,
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement(Badge, {
    variant: "outline",
    dot: true
  }, status === 'inprogress' ? 'In Progress' : status), /*#__PURE__*/React.createElement(Badge, {
    variant: "secondary"
  }, task.node), (task.labels || []).map(l => /*#__PURE__*/React.createElement(Badge, {
    key: l,
    variant: "outline"
  }, l)))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '14px 18px'
    }
  }, /*#__PURE__*/React.createElement(Tabs, {
    value: tab,
    onValueChange: setTab,
    tabs: [{
      value: 'diff',
      label: 'Diff'
    }, {
      value: 'logs',
      label: 'Logs'
    }, {
      value: 'attempts',
      label: 'Attempts'
    }]
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '0 18px 18px'
    }
  }, tab === 'diff' && /*#__PURE__*/React.createElement(DiffPanel, null), tab === 'logs' && /*#__PURE__*/React.createElement(LogsPanel, {
    node: task.node
  }), tab === 'attempts' && /*#__PURE__*/React.createElement(AttemptsPanel, null)), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: 16,
      borderTop: '1px solid var(--border)',
      display: 'flex',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(Button, {
    variant: "primary",
    size: "sm",
    style: {
      flex: 1
    }
  }, "Merge"), /*#__PURE__*/React.createElement(Button, {
    variant: "outline",
    size: "sm"
  }, "Rebase"), /*#__PURE__*/React.createElement(Button, {
    variant: "ghost",
    size: "sm"
  }, "Open in IDE"))));
}
function DiffPanel() {
  const lines = [{
    t: 'meta',
    s: 'src/auth/callback.ts'
  }, {
    t: 'ctx',
    s: '  export async function handleCallback(req) {'
  }, {
    t: 'del',
    s: "-   const token = req.query.code;"
  }, {
    t: 'add',
    s: "+   const token = await exchangeCode(req.query.code);"
  }, {
    t: 'add',
    s: "+   await persistSession(token);"
  }, {
    t: 'ctx',
    s: '    return redirect("/projects");'
  }, {
    t: 'ctx',
    s: '  }'
  }];
  const color = {
    meta: 'var(--text-muted)',
    ctx: 'var(--text-muted)',
    add: 'var(--console-success)',
    del: 'var(--console-error)'
  };
  const bg = {
    add: 'hsl(var(--vks-emerald-hsl) / 0.08)',
    del: 'hsl(var(--vks-coral-hsl) / 0.08)'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--console-bg)',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)',
      overflow: 'hidden',
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)'
    }
  }, lines.map((l, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      padding: '3px 12px',
      color: color[l.t],
      background: bg[l.t] || 'transparent',
      whiteSpace: 'pre',
      fontWeight: l.t === 'meta' ? 600 : 400
    }
  }, l.s)));
}
function LogsPanel({
  node
}) {
  const lines = [['muted', `→ connecting to node ${node}`], ['ok', '✓ worktree created · branch feat/auth'], ['fg', '$ claude-code run'], ['muted', '  reading 14 files…'], ['cy', '  editing src/auth/callback.ts'], ['ok', '✓ applied 2 edits'], ['err', '✗ test failed: expected session to persist']];
  const map = {
    muted: 'var(--text-muted)',
    ok: 'var(--console-success)',
    err: 'var(--console-error)',
    cy: 'var(--vks-cyan)',
    fg: 'var(--foreground)'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--console-bg)',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)',
      padding: '12px 14px',
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)',
      lineHeight: 1.7
    }
  }, lines.map((l, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      color: map[l[0]]
    }
  }, l[1])));
}
function AttemptsPanel() {
  const {
    Badge
  } = window.VKSwarmDesignSystem_067861;
  const items = [{
    agent: 'claude-code',
    state: 'running',
    when: 'now'
  }, {
    agent: 'codex',
    state: 'failed',
    when: '8m ago'
  }];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, items.map((a, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: 12,
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)'
    }
  }, a.state === 'running' ? /*#__PURE__*/React.createElement("span", {
    className: "vks-loader",
    style: {
      width: 14,
      height: 14
    }
  }) : /*#__PURE__*/React.createElement("span", {
    style: {
      width: 9,
      height: 9,
      borderRadius: '50%',
      background: 'var(--danger)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)',
      flex: 1
    }
  }, a.agent), /*#__PURE__*/React.createElement(Badge, {
    variant: "outline"
  }, a.state), /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 'var(--text-xs)',
      color: 'var(--text-dim)'
    }
  }, a.when))));
}
Object.assign(window, {
  NodesView,
  ProcessesView,
  TaskDrawer
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/vk-swarm-app/panels.jsx", error: String((e && e.message) || e) }); }

// ui_kits/vk-swarm-app/settings-panels.jsx
try { (() => {
// VK-Swarm UI kit — the remaining 7 settings panels (Projects, Organizations,
// Swarm, Agents, MCP, Webhooks, System). Each is a ({ draft, patch }) => cards
// of SettingsSection/SettingsRow. Registered on window.VKS_PANELS for
// settings.jsx to look up. Simple toggles/selects write to the shared draft so
// the dirty save-bar reacts; list add/remove use local ephemeral state.
const {
  useState: useStateP
} = React;
function _sel(opts) {
  return opts.map(o => typeof o === 'string' ? {
    value: o,
    label: o
  } : o);
}
function DS() {
  return window.VKSwarmDesignSystem_067861;
}

// ------------------------------------------------------------ Projects ------
function ProjectsPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Input,
    Switch,
    Checkbox
  } = DS();
  const p = draft.projects;
  const set = (k, v) => patch('projects', {
    [k]: v
  });
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Worktrees",
    description: "How isolated git worktrees are created for each attempt."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Default base branch",
    htmlFor: "p-branch",
    helper: "Branch new worktrees are cut from."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "p-branch",
    mono: true,
    value: p.defaultBranch,
    onChange: e => set('defaultBranch', e.target.value)
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Worktree directory",
    htmlFor: "p-wt",
    helper: "Where worktrees live on each node."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "p-wt",
    mono: true,
    value: p.worktreeBase,
    onChange: e => set('worktreeBase', e.target.value)
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Clean up merged worktrees",
    htmlFor: "p-clean",
    helper: "Remove the worktree once its task reaches Done."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "p-clean",
    checked: p.autoCleanup,
    onCheckedChange: v => set('autoCleanup', v)
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Copy .env into worktree",
    htmlFor: "p-env",
    helper: "Copy the repo\u2019s local env files into each worktree."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "p-env",
    checked: p.copyEnv,
    onCheckedChange: v => set('copyEnv', v)
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Setup script",
    description: "Runs once after a worktree is created, before the agent starts."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Command",
    htmlFor: "p-setup",
    helper: "e.g. install dependencies."
  }, /*#__PURE__*/React.createElement("textarea", {
    id: "p-setup",
    className: "vks-input vks-input--mono",
    rows: 3,
    value: p.setupScript,
    onChange: e => set('setupScript', e.target.value),
    style: {
      height: 'auto'
    }
  }))));
}

// ------------------------------------------------------- Organizations ------
function OrganizationsPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Switch,
    Badge
  } = DS();
  const o = draft.organizations;
  const set = (k, v) => patch('organizations', {
    [k]: v
  });
  const members = [{
    name: 'david',
    hint: 'owner · you',
    role: 'Owner',
    variant: 'secondary'
  }, {
    name: 'justX',
    hint: 'justX.raverx.net',
    role: 'Admin',
    variant: 'outline'
  }, {
    name: 'ci-bot',
    hint: 'service account',
    role: 'Member',
    variant: 'outline'
  }];
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Organization",
    description: "Settings apply to the selected organization."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Active organization",
    htmlFor: "o-org"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "o-org",
    value: o.org,
    onValueChange: v => set('org', v),
    options: _sel([{
      value: 'raverx',
      label: 'raverx'
    }, {
      value: 'acme-labs',
      label: 'acme-labs'
    }])
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Access",
    description: "Defaults for new members and merges."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Default member role",
    htmlFor: "o-role"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "o-role",
    value: o.defaultRole,
    onValueChange: v => set('defaultRole', v),
    options: _sel([{
      value: 'MEMBER',
      label: 'Member'
    }, {
      value: 'ADMIN',
      label: 'Admin'
    }, {
      value: 'VIEWER',
      label: 'Viewer'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Require review before merge",
    htmlFor: "o-review",
    helper: "A human must approve a diff before it can be merged."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "o-review",
    checked: o.requireReview,
    onCheckedChange: v => set('requireReview', v)
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Enforce SSO",
    htmlFor: "o-sso",
    helper: "Members must authenticate through your identity provider."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "o-sso",
    checked: o.sso,
    onCheckedChange: v => set('sso', v)
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Members",
    description: "3 members in this organization."
  }, /*#__PURE__*/React.createElement("div", {
    className: "vks-card",
    style: {
      overflow: 'hidden'
    }
  }, members.map((m, i) => /*#__PURE__*/React.createElement("div", {
    key: m.name,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '10px 14px',
      borderBottom: i < members.length - 1 ? '1px solid var(--border)' : 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 28,
      height: 28,
      borderRadius: '50%',
      background: 'var(--surface-raised)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)',
      flexShrink: 0
    }
  }, m.name.slice(0, 2)), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)'
    }
  }, m.name), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-dim)'
    }
  }, m.hint)), /*#__PURE__*/React.createElement(Badge, {
    variant: m.variant
  }, m.role))))));
}

// -------------------------------------------------------------- Swarm -------
function SwarmPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Switch,
    Badge,
    Button
  } = DS();
  const s = draft.swarm;
  const set = (k, v) => patch('swarm', {
    [k]: v
  });
  const [labels, setLabels] = useStateP(['frontend', 'infra', 'urgent', 'agent:claude']);
  const [draftLabel, setDraftLabel] = useStateP('');
  const addLabel = () => {
    const v = draftLabel.trim();
    if (v && !labels.includes(v)) setLabels(l => [...l, v]);
    setDraftLabel('');
  };
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Swarm",
    description: "Shared configuration synced across every node."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Organization",
    htmlFor: "sw-org"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "sw-org",
    value: s.org,
    onValueChange: v => set('org', v),
    options: _sel([{
      value: 'raverx',
      label: 'raverx'
    }, {
      value: 'acme-labs',
      label: 'acme-labs'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Auto-sync",
    htmlFor: "sw-sync",
    helper: "Push shared projects, labels and templates to nodes as they change."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "sw-sync",
    checked: s.autoSync,
    onCheckedChange: v => set('autoSync', v)
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Conflict strategy",
    htmlFor: "sw-conf",
    helper: "What to do when a node\u2019s local copy diverges."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "sw-conf",
    value: s.conflictStrategy,
    onValueChange: v => set('conflictStrategy', v),
    options: _sel([{
      value: 'MANUAL',
      label: 'Ask me'
    }, {
      value: 'HIVE_WINS',
      label: 'Hive wins'
    }, {
      value: 'NODE_WINS',
      label: 'Node wins'
    }])
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Shared labels",
    description: "Available on tasks across the whole swarm."
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexWrap: 'wrap',
      gap: 6
    }
  }, labels.map(l => /*#__PURE__*/React.createElement("span", {
    key: l,
    style: {
      display: 'inline-flex'
    }
  }, /*#__PURE__*/React.createElement(Badge, {
    variant: "outline"
  }, l, /*#__PURE__*/React.createElement("button", {
    onClick: () => setLabels(xs => xs.filter(x => x !== l)),
    "aria-label": 'Remove ' + l,
    style: {
      background: 'none',
      border: 0,
      color: 'var(--text-dim)',
      cursor: 'pointer',
      marginLeft: 6,
      padding: 0,
      lineHeight: 1
    }
  }, "\xD7"))))), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Add label",
    htmlFor: "sw-newlabel"
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }, /*#__PURE__*/React.createElement("input", {
    id: "sw-newlabel",
    className: "vks-input vks-input--mono",
    placeholder: "label name",
    value: draftLabel,
    onChange: e => setDraftLabel(e.target.value),
    onKeyDown: e => e.key === 'Enter' && addLabel()
  })), /*#__PURE__*/React.createElement(Button, {
    variant: "outline",
    size: "sm",
    onClick: addLabel
  }, "Add")))));
}

// -------------------------------------------------------------- Agents ------
function AgentsPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Input,
    Checkbox,
    Button,
    Badge
  } = DS();
  const a = draft.agents;
  const set = (k, v) => patch('agents', {
    [k]: v
  });
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Executor profiles",
    description: "Per-agent configuration profiles used when running attempts."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Agent",
    htmlFor: "ag-exec"
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr auto',
      gap: 8,
      alignItems: 'end'
    }
  }, /*#__PURE__*/React.createElement(Select, {
    id: "ag-exec",
    value: a.executor,
    onValueChange: v => set('executor', v),
    options: _sel([{
      value: 'CLAUDE_CODE',
      label: 'Claude Code'
    }, {
      value: 'CODEX',
      label: 'Codex'
    }, {
      value: 'OPENCODE',
      label: 'OpenCode'
    }, {
      value: 'GEMINI',
      label: 'Gemini'
    }])
  }), /*#__PURE__*/React.createElement(Select, {
    value: a.config,
    onValueChange: v => set('config', v),
    options: _sel([{
      value: 'DEFAULT',
      label: 'DEFAULT'
    }, {
      value: 'PLAN',
      label: 'PLAN'
    }, {
      value: 'YOLO',
      label: 'YOLO'
    }])
  }), /*#__PURE__*/React.createElement(Button, {
    variant: "destructive",
    size: "sm"
  }, /*#__PURE__*/React.createElement(window.Icon, {
    d: window.SICONS.trash,
    size: 14
  })))), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Edit as raw JSON",
    htmlFor: "ag-json",
    helper: "Switch off the form editor to edit profiles.json directly."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "ag-json",
    checked: !a.formEditor,
    onCheckedChange: v => set('formEditor', !v)
  })), a.formEditor ? /*#__PURE__*/React.createElement("div", {
    className: "vks-settings__body",
    style: {
      gap: 'var(--space-4)',
      paddingTop: 4,
      borderTop: '1px dashed var(--border)'
    }
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Model",
    htmlFor: "ag-model"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "ag-model",
    defaultValue: "sonnet",
    options: _sel([{
      value: 'sonnet',
      label: 'claude-sonnet-4.5'
    }, {
      value: 'opus',
      label: 'claude-opus-4'
    }, {
      value: 'haiku',
      label: 'claude-haiku-4'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Extra CLI args",
    htmlFor: "ag-args",
    helper: "Appended to the executor invocation."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "ag-args",
    mono: true,
    defaultValue: "--dangerously-skip-permissions"
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Sandbox",
    htmlFor: "ag-sandbox",
    helper: "Run the agent in a restricted filesystem sandbox."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "ag-sandbox",
    defaultChecked: true
  }))) : /*#__PURE__*/React.createElement(SettingsRow, {
    label: "profiles.json",
    htmlFor: "ag-raw",
    helper: "~/.vk-swarm/profiles.json"
  }, /*#__PURE__*/React.createElement("textarea", {
    id: "ag-raw",
    className: "vks-input vks-input--mono",
    rows: 7,
    style: {
      height: 'auto'
    },
    defaultValue: '{\n  "executors": {\n    "CLAUDE_CODE": {\n      "DEFAULT": { "model": "claude-sonnet-4.5" }\n    }\n  }\n}'
  }))));
}

// ---------------------------------------------------------------- MCP -------
function McpPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Switch,
    Button,
    Badge
  } = DS();
  const m = draft.mcp;
  const set = (k, v) => patch('mcp', {
    [k]: v
  });
  const [servers, setServers] = useStateP([{
    id: 'fs',
    name: 'filesystem',
    cmd: 'npx @modelcontextprotocol/server-filesystem',
    on: true
  }, {
    id: 'gh',
    name: 'github',
    cmd: 'npx @modelcontextprotocol/server-github',
    on: true
  }, {
    id: 'pg',
    name: 'postgres',
    cmd: 'uvx mcp-server-postgres',
    on: false
  }]);
  const toggle = id => setServers(s => s.map(x => x.id === id ? {
    ...x,
    on: !x.on
  } : x));
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Model Context Protocol",
    description: "MCP servers exposed to every agent in the swarm."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Scope",
    htmlFor: "mcp-scope",
    helper: "Where these servers apply."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "mcp-scope",
    value: m.scope,
    onValueChange: v => set('scope', v),
    options: _sel([{
      value: 'GLOBAL',
      label: 'All projects'
    }, {
      value: 'PROJECT',
      label: 'This project only'
    }])
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, servers.map(sv => /*#__PURE__*/React.createElement("div", {
    key: sv.id,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '12px 14px',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      minWidth: 0,
      flex: 1
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)',
      fontWeight: 500
    }
  }, sv.name), /*#__PURE__*/React.createElement(Badge, {
    variant: sv.on ? 'secondary' : 'outline',
    dot: sv.on
  }, sv.on ? 'connected' : 'disabled')), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-dim)',
      marginTop: 3,
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, sv.cmd)), /*#__PURE__*/React.createElement(Switch, {
    checked: sv.on,
    onCheckedChange: () => toggle(sv.id)
  })))), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(Button, {
    variant: "outline",
    size: "sm"
  }, /*#__PURE__*/React.createElement(window.Icon, {
    d: window.ICONS.plus,
    size: 14
  }), " Add server"))));
}

// ----------------------------------------------------------- Webhooks -------
function WebhooksPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Input,
    Switch,
    Checkbox,
    Button
  } = DS();
  const w = draft.webhooks;
  const set = (k, v) => patch('webhooks', {
    [k]: v
  });
  const events = [['task.created', 'A task is added to the board'], ['attempt.finished', 'An agent attempt completes'], ['review.requested', 'A diff enters In Review'], ['task.merged', 'A task is merged and closed']];
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Delivery",
    description: "Send swarm events to an external endpoint."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Enable webhooks",
    htmlFor: "wh-on",
    helper: "Deliver events over HTTPS POST."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "wh-on",
    checked: w.enabled,
    onCheckedChange: v => set('enabled', v)
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Endpoint URL",
    htmlFor: "wh-url",
    helper: "Receives a signed JSON payload per event."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "wh-url",
    mono: true,
    defaultValue: "https://hooks.raverx.net/vk-swarm",
    disabled: !w.enabled
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Signing secret",
    htmlFor: "wh-secret",
    helper: "Used to verify the X-VK-Signature header."
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }, /*#__PURE__*/React.createElement(Input, {
    id: "wh-secret",
    mono: true,
    type: "password",
    defaultValue: "whsec_9a3f2c1b7e",
    disabled: !w.enabled
  })), /*#__PURE__*/React.createElement(Button, {
    variant: "outline",
    size: "sm",
    disabled: !w.enabled
  }, /*#__PURE__*/React.createElement(window.Icon, {
    d: window.SICONS.refresh,
    size: 14
  }), " Rotate"))), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Retry attempts",
    htmlFor: "wh-retry",
    helper: "Failed deliveries are retried with backoff."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "wh-retry",
    value: w.retry,
    onValueChange: v => set('retry', v),
    options: _sel([{
      value: '0',
      label: 'No retries'
    }, {
      value: '3',
      label: '3 attempts'
    }, {
      value: '5',
      label: '5 attempts'
    }])
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Events",
    description: "Which events trigger a delivery."
  }, events.map(([id, desc], i) => /*#__PURE__*/React.createElement(SettingsRow, {
    key: id,
    inline: true,
    label: /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-code)',
        fontSize: 'var(--text-sm)'
      }
    }, id),
    htmlFor: 'wh-ev-' + i,
    helper: desc
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: 'wh-ev-' + i,
    defaultChecked: i < 3,
    disabled: !w.enabled
  })))));
}

// -------------------------------------------------------------- System ------
function SystemPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Switch,
    Button,
    Badge
  } = DS();
  const sy = draft.system;
  const set = (k, v) => patch('system', {
    [k]: v
  });
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Backups",
    description: "Snapshots of the hive database.",
    footer: /*#__PURE__*/React.createElement(Button, {
      variant: "outline",
      size: "sm"
    }, /*#__PURE__*/React.createElement(window.Icon, {
      d: window.SICONS.database,
      size: 14
    }), " Back up now")
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Automatic backups",
    htmlFor: "sy-auto",
    helper: "Snapshot the hive on a schedule."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "sy-auto",
    checked: sy.autoBackup,
    onCheckedChange: v => set('autoBackup', v)
  })), sy.autoBackup && /*#__PURE__*/React.createElement(SettingsRow, {
    nested: true,
    label: "Interval",
    htmlFor: "sy-int"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "sy-int",
    value: sy.backupInterval,
    onValueChange: v => set('backupInterval', v),
    options: _sel([{
      value: 'HOURLY',
      label: 'Hourly'
    }, {
      value: 'DAILY',
      label: 'Daily'
    }, {
      value: 'WEEKLY',
      label: 'Weekly'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Retention",
    htmlFor: "sy-ret",
    helper: "Older snapshots are pruned."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "sy-ret",
    value: sy.retention,
    onValueChange: v => set('retention', v),
    options: _sel([{
      value: '7',
      label: '7 days'
    }, {
      value: '30',
      label: '30 days'
    }, {
      value: '90',
      label: '90 days'
    }])
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Hive sync",
    description: "Connection to the central hive."
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '12px 14px',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    className: "vks-node__pulse",
    style: {
      flexShrink: 0
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontSize: 'var(--text-sm)',
      fontWeight: 500
    }
  }, "Connected \xB7 wss://hive.raverx.net"), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-dim)',
      marginTop: 2
    }
  }, "4 nodes synced \xB7 last sync 12s ago")), /*#__PURE__*/React.createElement(Badge, {
    variant: "secondary",
    dot: true
  }, "online"))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Build info",
    description: "This VK-Swarm instance."
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: 'auto 1fr',
      rowGap: 8,
      columnGap: 16,
      fontSize: 'var(--text-sm)'
    }
  }, [['Version', 'v0.7.3'], ['Commit', '648692a'], ['Rust', '1.83.0'], ['Node registry', '4 nodes']].map(([k, v]) => /*#__PURE__*/React.createElement(React.Fragment, {
    key: k
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-muted)'
    }
  }, k), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--foreground)'
    }
  }, v)))), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Send anonymous telemetry",
    htmlFor: "sy-tel",
    helper: "Share crash reports and usage metrics to improve VK-Swarm."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "sy-tel",
    checked: sy.telemetry,
    onCheckedChange: v => set('telemetry', v)
  }))));
}
window.VKS_PANELS = {
  projects: ProjectsPanel,
  organizations: OrganizationsPanel,
  swarm: SwarmPanel,
  agents: AgentsPanel,
  mcp: McpPanel,
  webhooks: WebhooksPanel,
  system: SystemPanel
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/vk-swarm-app/settings-panels.jsx", error: String((e && e.message) || e) }); }

// ui_kits/vk-swarm-app/settings.jsx
try { (() => {
// VK-Swarm UI kit — full-page Settings route.
// Left sidebar (8 sections) + close/ESC, content is a stack of SettingsSection
// cards built from the design-system SettingsRow controls. A single draft
// object drives a sticky dirty save-bar shared across panels.
const {
  useState,
  useEffect,
  useRef
} = React;

// ---- Extra lucide-style icons not in chrome.jsx ----------------------------
const SICONS = {
  x: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M18 6 6 18M6 6l12 12"
  })),
  building: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M6 22V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v18Z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M6 12H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M10 6h4M10 10h4M10 14h4M10 18h4"
  })),
  network: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "9",
    y: "2",
    width: "6",
    height: "6",
    rx: "1"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "2",
    y: "16",
    width: "6",
    height: "6",
    rx: "1"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "16",
    y: "16",
    width: "6",
    height: "6",
    rx: "1"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 8v4M5 16v-2h14v2"
  })),
  cpu: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: "6",
    y: "6",
    width: "12",
    height: "12",
    rx: "2"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "9",
    y: "9",
    width: "6",
    height: "6"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M9 2v2M15 2v2M9 20v2M15 20v2M2 9h2M2 15h2M20 9h2M20 15h2"
  })),
  database: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("ellipse", {
    cx: "12",
    cy: "5",
    rx: "8",
    ry: "3"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"
  })),
  volume: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M11 5 6 9H2v6h4l5 4z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M15.5 8.5a5 5 0 0 1 0 7"
  })),
  check: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M20 6 9 17l-5-5"
  })),
  alert: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M12 9v4M12 17h.01"
  })),
  trash: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"
  })),
  refresh: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: "M3 12a9 9 0 0 1 15-6.7L21 8"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M21 3v5h-5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M21 12a9 9 0 0 1-15 6.7L3 16"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M3 21v-5h5"
  })),
  key: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: "7.5",
    cy: "15.5",
    r: "4.5"
  }), /*#__PURE__*/React.createElement("path", {
    d: "M10.7 12.3 21 2M16 7l3 3M14 9l3 3"
  }))
};

// ---- Nav model -------------------------------------------------------------
function navItems(ICONS) {
  return [{
    id: 'general',
    icon: ICONS.settings,
    name: 'General',
    desc: 'Appearance, editor, notifications'
  }, {
    id: 'projects',
    icon: ICONS.folder,
    name: 'Projects',
    desc: 'Per-project defaults'
  }, {
    id: 'organizations',
    icon: SICONS.building,
    name: 'Organizations',
    desc: 'Members & access'
  }, {
    id: 'swarm',
    icon: SICONS.network,
    name: 'Swarm',
    desc: 'Shared projects, labels, templates'
  }, {
    id: 'agents',
    icon: SICONS.cpu,
    name: 'Agents',
    desc: 'Executor profiles & configs'
  }, {
    id: 'mcp',
    icon: ICONS.server,
    name: 'MCP',
    desc: 'Model Context Protocol servers'
  }, {
    id: 'webhooks',
    icon: ICONS.bolt,
    name: 'Webhooks',
    desc: 'Outbound event delivery'
  }, {
    id: 'system',
    icon: SICONS.database,
    name: 'System',
    desc: 'Backups, hive sync, build info'
  }];
}

// ---- Default draft ---------------------------------------------------------
const DEFAULT_DRAFT = {
  general: {
    theme: 'DARK',
    language: 'BROWSER',
    uiFont: 'INTER',
    codeFont: 'JET_BRAINS_MONO',
    proseFont: 'SOURCE_SERIF',
    disableLigatures: false,
    executor: 'CLAUDE_CODE',
    variant: 'DEFAULT',
    editorType: 'VS_CODE',
    customCommand: '',
    remoteHost: '',
    remoteUser: '',
    terminalFontSize: '14',
    cursorBlink: true,
    timezone: 'LOCAL',
    tokenTs: false,
    tokenTsFormat: '[HH:mm:ss.SSS dd/MM/yyyy]',
    branchPrefix: 'vk',
    soundEnabled: true,
    soundFile: 'ROOK',
    pushEnabled: false,
    initialLoad: '100'
  },
  projects: {
    defaultBranch: 'main',
    worktreeBase: '~/.vk-swarm/worktrees',
    autoCleanup: true,
    copyEnv: true,
    setupScript: 'pnpm install'
  },
  organizations: {
    org: 'raverx',
    defaultRole: 'MEMBER',
    requireReview: true,
    sso: false
  },
  swarm: {
    org: 'raverx',
    autoSync: true,
    conflictStrategy: 'MANUAL'
  },
  agents: {
    executor: 'CLAUDE_CODE',
    config: 'DEFAULT',
    formEditor: true
  },
  mcp: {
    scope: 'GLOBAL'
  },
  webhooks: {
    enabled: true,
    retry: '3',
    secretSet: true
  },
  system: {
    autoBackup: true,
    backupInterval: 'DAILY',
    retention: '30',
    telemetry: false
  }
};

// ---- Small building blocks -------------------------------------------------
function Alert({
  variant,
  icon,
  children
}) {
  return /*#__PURE__*/React.createElement("div", {
    className: ['vks-alert', variant && 'vks-alert--' + variant].filter(Boolean).join(' '),
    role: "status"
  }, icon && /*#__PURE__*/React.createElement("span", {
    className: "vks-alert__icon"
  }, icon), /*#__PURE__*/React.createElement("div", null, children));
}
function sel(opts) {
  return opts.map(o => typeof o === 'string' ? {
    value: o,
    label: o
  } : o);
}

// ---- General panel (the flagship — full fidelity) --------------------------
function GeneralPanel({
  draft,
  patch
}) {
  const {
    SettingsSection,
    SettingsRow,
    Select,
    Input,
    Switch,
    Checkbox,
    Button
  } = window.VKSwarmDesignSystem_067861;
  const Icon = window.Icon;
  const g = draft.general;
  const set = (k, v) => patch('general', {
    [k]: v
  });
  const prefixError = (() => {
    const p = g.branchPrefix;
    if (!p) return null;
    if (/\s/.test(p)) return 'No spaces allowed.';
    if (p.includes('/')) return 'Cannot contain a slash.';
    if (p.startsWith('.')) return 'Cannot start with a dot.';
    return null;
  })();
  const remoteCapable = ['VS_CODE', 'CURSOR', 'WINDSURF'].includes(g.editorType);
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Appearance",
    description: "Customize how VK-Swarm looks on this device."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Theme",
    htmlFor: "s-theme",
    helper: "Midnight Terminal is the default. Applied on save."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-theme",
    value: g.theme,
    onValueChange: v => set('theme', v),
    options: sel([{
      value: 'DARK',
      label: 'Dark'
    }, {
      value: 'LIGHT',
      label: 'Light'
    }, {
      value: 'SYSTEM',
      label: 'System'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Language",
    htmlFor: "s-lang",
    helper: "Interface language for menus and labels."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-lang",
    value: g.language,
    onValueChange: v => set('language', v),
    options: sel([{
      value: 'BROWSER',
      label: 'Browser Default'
    }, {
      value: 'EN',
      label: 'English'
    }, {
      value: 'DE',
      label: 'Deutsch'
    }, {
      value: 'JA',
      label: '日本語'
    }])
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Typography",
    description: "Fonts for the interface, code and prose."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "UI font",
    htmlFor: "s-uifont"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-uifont",
    value: g.uiFont,
    onValueChange: v => set('uiFont', v),
    options: sel([{
      value: 'INTER',
      label: 'Inter'
    }, {
      value: 'ROBOTO',
      label: 'Roboto'
    }, {
      value: 'PUBLIC_SANS',
      label: 'Public Sans'
    }, {
      value: 'CHIVO_MONO',
      label: 'Chivo Mono'
    }, {
      value: 'SYSTEM',
      label: 'System'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Code font",
    htmlFor: "s-codefont",
    helper: "Logs, diffs, branches and node names."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-codefont",
    value: g.codeFont,
    onValueChange: v => set('codeFont', v),
    options: sel([{
      value: 'JET_BRAINS_MONO',
      label: 'JetBrains Mono'
    }, {
      value: 'CASCADIA_MONO',
      label: 'Cascadia Mono'
    }, {
      value: 'HACK',
      label: 'Hack'
    }, {
      value: 'IBM_PLEX_MONO',
      label: 'IBM Plex Mono'
    }, {
      value: 'SYSTEM',
      label: 'System'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Prose font",
    htmlFor: "s-prosefont"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-prosefont",
    value: g.proseFont,
    onValueChange: v => set('proseFont', v),
    options: sel([{
      value: 'SOURCE_SERIF',
      label: 'Source Serif 4'
    }, {
      value: 'INTER',
      label: 'Inter'
    }, {
      value: 'GEORGIA',
      label: 'Georgia'
    }, {
      value: 'SYSTEM',
      label: 'System'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Disable ligatures",
    htmlFor: "s-liga",
    helper: "Render code fonts without programming ligatures."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "s-liga",
    checked: g.disableLigatures,
    onCheckedChange: v => set('disableLigatures', v)
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-sm)',
      color: 'var(--text-muted)',
      background: 'var(--surface-raised)',
      border: '1px solid var(--border)',
      borderRadius: 'var(--radius-md)',
      padding: '10px 12px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-dim)'
    }
  }, "preview\xA0"), "const worktree = await hive.", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--vks-cyan)'
    }
  }, "spawn"), "(node);  ", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--console-success)'
    }
  }, '// => ok'))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Task Execution",
    description: "Default coding agent for new task attempts."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Executor",
    htmlFor: "s-exec",
    helper: "Availability is checked when an attempt starts."
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-exec",
    value: g.executor,
    onValueChange: v => set('executor', v),
    options: sel([{
      value: 'CLAUDE_CODE',
      label: 'Claude Code'
    }, {
      value: 'CODEX',
      label: 'Codex'
    }, {
      value: 'OPENCODE',
      label: 'OpenCode'
    }, {
      value: 'GEMINI',
      label: 'Gemini'
    }])
  }), /*#__PURE__*/React.createElement(Select, {
    value: g.variant,
    onValueChange: v => set('variant', v),
    options: sel([{
      value: 'DEFAULT',
      label: 'Default'
    }, {
      value: 'PLAN',
      label: 'Plan'
    }, {
      value: 'ROUTER',
      label: 'Router'
    }])
  })))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Editor",
    description: "Opens worktrees in your local IDE."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Preferred editor",
    htmlFor: "s-editor",
    helper: "Used by \u201COpen in IDE\u201D."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-editor",
    value: g.editorType,
    onValueChange: v => set('editorType', v),
    options: sel([{
      value: 'VS_CODE',
      label: 'VS Code'
    }, {
      value: 'CURSOR',
      label: 'Cursor'
    }, {
      value: 'ZED',
      label: 'Zed'
    }, {
      value: 'WINDSURF',
      label: 'Windsurf'
    }, {
      value: 'INTELLIJ',
      label: 'IntelliJ'
    }, {
      value: 'CUSTOM',
      label: 'Custom'
    }])
  })), g.editorType === 'CUSTOM' && /*#__PURE__*/React.createElement(SettingsRow, {
    nested: true,
    label: "Custom command",
    htmlFor: "s-cmd",
    helper: "Receives the worktree path as its final argument."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "s-cmd",
    mono: true,
    value: g.customCommand,
    placeholder: "e.g. code --wait",
    onChange: e => set('customCommand', e.target.value)
  })), remoteCapable && /*#__PURE__*/React.createElement(SettingsRow, {
    nested: true,
    label: "Remote SSH host",
    htmlFor: "s-ssh",
    helper: "Open worktrees over Remote-SSH. Leave blank for local."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "s-ssh",
    mono: true,
    value: g.remoteHost,
    placeholder: "e.g. justX.raverx.net",
    onChange: e => set('remoteHost', e.target.value)
  })), remoteCapable && g.remoteHost && /*#__PURE__*/React.createElement(SettingsRow, {
    nested: true,
    label: "Remote SSH user",
    htmlFor: "s-sshuser"
  }, /*#__PURE__*/React.createElement(Input, {
    id: "s-sshuser",
    mono: true,
    value: g.remoteUser,
    placeholder: "e.g. david",
    onChange: e => set('remoteUser', e.target.value)
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Terminal",
    description: "Appearance of the embedded log/terminal viewer.",
    footer: /*#__PURE__*/React.createElement(Button, {
      variant: "outline",
      size: "sm",
      onClick: () => patch('general', {
        terminalFontSize: '14',
        cursorBlink: true
      })
    }, "Reset to defaults")
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Font size",
    htmlFor: "s-tsize"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-tsize",
    value: g.terminalFontSize,
    onValueChange: v => set('terminalFontSize', v),
    options: sel([{
      value: '10',
      label: '10px'
    }, {
      value: '12',
      label: '12px'
    }, {
      value: '14',
      label: '14px (default)'
    }, {
      value: '16',
      label: '16px'
    }, {
      value: '18',
      label: '18px'
    }, {
      value: '20',
      label: '20px'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Cursor blink",
    htmlFor: "s-blink",
    helper: "Blink the terminal cursor."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "s-blink",
    checked: g.cursorBlink,
    onCheckedChange: v => set('cursorBlink', v)
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Timestamps",
    description: "How times are shown in logs and activity."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Timezone",
    htmlFor: "s-tz"
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-tz",
    value: g.timezone,
    onValueChange: v => set('timezone', v),
    options: sel([{
      value: 'LOCAL',
      label: 'Local time'
    }, {
      value: 'UTC',
      label: 'UTC'
    }, {
      value: 'America/Los_Angeles',
      label: 'America/Los_Angeles (PST/PDT)'
    }, {
      value: 'Europe/London',
      label: 'Europe/London (GMT/BST)'
    }, {
      value: 'Australia/Sydney',
      label: 'Australia/Sydney (AEST/AEDT)'
    }, {
      value: 'Asia/Tokyo',
      label: 'Asia/Tokyo (JST)'
    }])
  })), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Token-level timestamps",
    htmlFor: "s-tokents",
    helper: "Prefix each streamed token with a timestamp."
  }, /*#__PURE__*/React.createElement(Switch, {
    id: "s-tokents",
    checked: g.tokenTs,
    onCheckedChange: v => set('tokenTs', v)
  })), g.tokenTs && /*#__PURE__*/React.createElement(SettingsRow, {
    nested: true,
    label: "Timestamp format",
    htmlFor: "s-tokenfmt",
    helper: "date-fns tokens. Applied to the token prefix."
  }, /*#__PURE__*/React.createElement(Input, {
    id: "s-tokenfmt",
    mono: true,
    value: g.tokenTsFormat,
    onChange: e => set('tokenTsFormat', e.target.value)
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Git",
    description: "Defaults applied when agents create branches."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Branch prefix",
    htmlFor: "s-prefix",
    error: prefixError,
    helper: /*#__PURE__*/React.createElement(React.Fragment, null, "Prepended to generated branch names. Preview: ", /*#__PURE__*/React.createElement("span", {
      className: "vks-field__preview"
    }, (g.branchPrefix ? g.branchPrefix + '/' : '') + 'feat-auth-callback'))
  }, /*#__PURE__*/React.createElement(Input, {
    id: "s-prefix",
    mono: true,
    value: g.branchPrefix,
    placeholder: "e.g. vk",
    "aria-invalid": !!prefixError,
    style: prefixError ? {
      borderColor: 'var(--danger)'
    } : undefined,
    onChange: e => set('branchPrefix', e.target.value.trim())
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Notifications",
    description: "Alerts when attempts finish or need review."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Sound",
    htmlFor: "s-sound",
    helper: "Play a sound when an attempt completes."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "s-sound",
    checked: g.soundEnabled,
    onCheckedChange: v => set('soundEnabled', v)
  })), g.soundEnabled && /*#__PURE__*/React.createElement(SettingsRow, {
    nested: true,
    label: "Sound file",
    htmlFor: "s-soundfile"
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-soundfile",
    value: g.soundFile,
    onValueChange: v => set('soundFile', v),
    options: sel([{
      value: 'ROOK',
      label: 'Rook'
    }, {
      value: 'ABSTRACT_SOUND',
      label: 'Abstract Sound'
    }, {
      value: 'COW_MOOING',
      label: 'Cow Mooing'
    }, {
      value: 'PHONE_VIBRATION',
      label: 'Phone Vibration'
    }])
  })), /*#__PURE__*/React.createElement(Button, {
    variant: "outline",
    size: "icon",
    title: "Preview sound"
  }, /*#__PURE__*/React.createElement(Icon, {
    d: SICONS.volume,
    size: 16
  })))), /*#__PURE__*/React.createElement(SettingsRow, {
    inline: true,
    label: "Push notifications",
    htmlFor: "s-push",
    helper: "Notify even when the tab is in the background."
  }, /*#__PURE__*/React.createElement(Checkbox, {
    id: "s-push",
    checked: g.pushEnabled,
    onCheckedChange: v => set('pushEnabled', v)
  }))), /*#__PURE__*/React.createElement(SettingsSection, {
    title: "Performance",
    description: "Tune how much history loads up front."
  }, /*#__PURE__*/React.createElement(SettingsRow, {
    label: "Initial log load",
    htmlFor: "s-load",
    helper: "Older entries load on scroll."
  }, /*#__PURE__*/React.createElement(Select, {
    id: "s-load",
    value: g.initialLoad,
    onValueChange: v => set('initialLoad', v),
    options: sel([{
      value: '50',
      label: '50 entries'
    }, {
      value: '100',
      label: '100 entries (default)'
    }, {
      value: '200',
      label: '200 entries'
    }, {
      value: '500',
      label: '500 entries'
    }])
  }))));
}

// ---- Layout + state --------------------------------------------------------
function SettingsView({
  onClose,
  onApplyTheme
}) {
  const {
    Button
  } = window.VKSwarmDesignSystem_067861;
  const Icon = window.Icon;
  const ICONS = window.ICONS;
  const bp = window.useBreakpoint();
  const compact = bp !== 'desktop';
  const [active, setActive] = useState('general');
  const [draft, setDraft] = useState(() => JSON.parse(JSON.stringify(DEFAULT_DRAFT)));
  const [saved, setSaved] = useState(() => JSON.parse(JSON.stringify(DEFAULT_DRAFT)));
  const [success, setSuccess] = useState(false);
  const successTimer = useRef(null);
  const patch = (panel, partial) => setDraft(d => ({
    ...d,
    [panel]: {
      ...d[panel],
      ...partial
    }
  }));
  const dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  useEffect(() => {
    const onKey = e => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  useEffect(() => () => successTimer.current && clearTimeout(successTimer.current), []);
  const save = () => {
    setSaved(JSON.parse(JSON.stringify(draft)));
    onApplyTheme && onApplyTheme(draft.general.theme);
    setSuccess(true);
    successTimer.current && clearTimeout(successTimer.current);
    successTimer.current = setTimeout(() => setSuccess(false), 2600);
  };
  const discard = () => setDraft(JSON.parse(JSON.stringify(saved)));
  const items = navItems(ICONS);
  const current = items.find(i => i.id === active);
  const PANELS = window.VKS_PANELS || {};
  const Panel = active === 'general' ? GeneralPanel : PANELS[active];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      zIndex: 20,
      background: 'var(--background)',
      display: 'flex',
      flexDirection: 'column'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '0 20px',
      height: 56,
      borderBottom: '1px solid var(--border)',
      flexShrink: 0
    }
  }, /*#__PURE__*/React.createElement("h1", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'var(--text-2xl)',
      fontWeight: 600,
      margin: 0
    }
  }, "Settings"), /*#__PURE__*/React.createElement("button", {
    className: "vks-btn vks-btn--ghost vks-btn--sm",
    onClick: onClose,
    style: {
      gap: 6,
      border: '1px solid var(--border-strong)'
    },
    title: "Close settings"
  }, /*#__PURE__*/React.createElement(Icon, {
    d: SICONS.x,
    size: 15
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-code)',
      fontSize: 'var(--text-xs)',
      color: 'var(--text-muted)'
    }
  }, "ESC"))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      display: 'flex',
      minHeight: 0,
      flexDirection: compact ? 'column' : 'row'
    }
  }, /*#__PURE__*/React.createElement("aside", {
    style: {
      width: compact ? 'auto' : 248,
      flexShrink: 0,
      borderRight: compact ? 0 : '1px solid var(--border)',
      borderBottom: compact ? '1px solid var(--border)' : 0,
      padding: compact ? '8px 12px' : '16px 12px',
      overflowX: compact ? 'auto' : 'visible',
      overflowY: compact ? 'hidden' : 'auto'
    }
  }, /*#__PURE__*/React.createElement("nav", {
    style: {
      display: 'flex',
      flexDirection: compact ? 'row' : 'column',
      gap: compact ? 6 : 2
    }
  }, items.map(it => {
    const on = it.id === active;
    return /*#__PURE__*/React.createElement("button", {
      key: it.id,
      onClick: () => setActive(it.id),
      style: {
        display: 'flex',
        alignItems: 'flex-start',
        gap: 11,
        textAlign: 'left',
        width: compact ? 'auto' : '100%',
        padding: compact ? '8px 12px' : '9px 11px',
        borderRadius: 'var(--radius-md)',
        cursor: 'pointer',
        border: compact && on ? '1px solid var(--primary)' : '1px solid transparent',
        background: on ? 'var(--surface-raised)' : 'transparent',
        color: on ? 'var(--foreground)' : 'var(--text-muted)',
        whiteSpace: 'nowrap',
        transition: 'background-color .15s ease, color .15s ease'
      },
      onMouseEnter: e => {
        if (!on) e.currentTarget.style.background = 'var(--surface-card)';
      },
      onMouseLeave: e => {
        if (!on) e.currentTarget.style.background = 'transparent';
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        color: on ? 'var(--primary)' : 'var(--text-muted)',
        marginTop: 1,
        flexShrink: 0,
        display: 'flex'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      d: it.icon,
      size: 16
    })), /*#__PURE__*/React.createElement("span", {
      style: {
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        fontSize: 'var(--text-sm)',
        fontWeight: 500
      }
    }, it.name), !compact && /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        fontSize: 'var(--text-xs)',
        color: 'var(--text-dim)',
        marginTop: 1
      }
    }, it.desc)));
  }))), /*#__PURE__*/React.createElement("main", {
    style: {
      flex: 1,
      minWidth: 0,
      overflowY: 'auto',
      padding: compact ? '20px 16px 8px' : '28px 32px 12px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 720,
      margin: '0 auto',
      display: 'flex',
      flexDirection: 'column',
      gap: 20
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'var(--text-xl)',
      fontWeight: 600,
      margin: 0
    }
  }, current.name), /*#__PURE__*/React.createElement("p", {
    style: {
      fontSize: 'var(--text-sm)',
      color: 'var(--text-muted)',
      margin: '4px 0 0'
    }
  }, current.desc)), success && /*#__PURE__*/React.createElement(Alert, {
    variant: "success",
    icon: /*#__PURE__*/React.createElement(Icon, {
      d: SICONS.check,
      size: 16
    })
  }, "Settings saved."), Panel ? /*#__PURE__*/React.createElement(Panel, {
    draft: draft,
    patch: patch
  }) : /*#__PURE__*/React.createElement(Alert, {
    variant: "info",
    icon: /*#__PURE__*/React.createElement(Icon, {
      d: SICONS.alert,
      size: 16
    })
  }, "This panel isn\u2019t wired yet."), dirty && /*#__PURE__*/React.createElement("div", {
    className: "vks-savebar"
  }, /*#__PURE__*/React.createElement("span", {
    className: "vks-savebar__hint"
  }, /*#__PURE__*/React.createElement("span", {
    className: "vks-savebar__dot"
  }), "Unsaved changes"), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(Button, {
    variant: "ghost",
    size: "sm",
    onClick: discard
  }, "Discard"), /*#__PURE__*/React.createElement(Button, {
    variant: "primary",
    size: "sm",
    onClick: save
  }, "Save changes"))), /*#__PURE__*/React.createElement("div", {
    style: {
      height: 8
    }
  })))));
}
Object.assign(window, {
  SettingsView,
  SICONS
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/vk-swarm-app/settings.jsx", error: String((e && e.message) || e) }); }

__ds_ns.NodeCard = __ds_scope.NodeCard;

__ds_ns.StatusBadge = __ds_scope.StatusBadge;

__ds_ns.TaskCard = __ds_scope.TaskCard;

__ds_ns.Badge = __ds_scope.Badge;

__ds_ns.Button = __ds_scope.Button;

__ds_ns.Card = __ds_scope.Card;

__ds_ns.CardHeader = __ds_scope.CardHeader;

__ds_ns.CardTitle = __ds_scope.CardTitle;

__ds_ns.CardDescription = __ds_scope.CardDescription;

__ds_ns.CardContent = __ds_scope.CardContent;

__ds_ns.CardFooter = __ds_scope.CardFooter;

__ds_ns.Checkbox = __ds_scope.Checkbox;

__ds_ns.Input = __ds_scope.Input;

__ds_ns.Loader = __ds_scope.Loader;

__ds_ns.Select = __ds_scope.Select;

__ds_ns.Switch = __ds_scope.Switch;

__ds_ns.Tabs = __ds_scope.Tabs;

__ds_ns.SettingsRow = __ds_scope.SettingsRow;

__ds_ns.SettingsSection = __ds_scope.SettingsSection;

})();
