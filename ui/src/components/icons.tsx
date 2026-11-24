import type { SVGProps } from "react";

type P = SVGProps<SVGSVGElement>;

const base = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
};

export const IconOverview = (p: P) => (
  <svg {...base} {...p}>
    <rect x="1.75" y="5.25" width="12.5" height="5.5" rx="2.75" />
    <path d="M6.5 5.25v5.5M10 5.25v5.5" />
  </svg>
);

export const IconSystem = (p: P) => (
  <svg {...base} {...p}>
    <path d="M8 1.75 13.25 4v4c0 3-2.25 5.25-5.25 6.25C5 13.25 2.75 11 2.75 8V4L8 1.75Z" />
    <path d="M5.75 8.25 7.25 9.75 10.25 6.5" />
  </svg>
);

export const IconMinimize = (p: P) => (
  <svg {...base} width={12} height={12} viewBox="0 0 12 12" {...p}>
    <path d="M2.5 6h7" />
  </svg>
);

export const IconMaximize = (p: P) => (
  <svg {...base} width={12} height={12} viewBox="0 0 12 12" {...p}>
    <rect x="2.5" y="2.5" width="7" height="7" rx="1.5" />
  </svg>
);

export const IconClose = (p: P) => (
  <svg {...base} width={12} height={12} viewBox="0 0 12 12" {...p}>
    <path d="m3 3 6 6M9 3 3 9" />
  </svg>
);

export const IconRetry = (p: P) => (
  <svg {...base} {...p}>
    <path d="M13.25 8A5.25 5.25 0 1 1 11.6 4.2" />
    <path d="M13.25 2.5v3h-3" />
  </svg>
);

export const IconChevron = (p: P) => (
  <svg {...base} width={10} height={10} viewBox="0 0 10 10" strokeWidth={1.6} {...p}>
    <path d="m3.5 2 3 3-3 3" />
  </svg>
);

export const IconBolt = (p: P) => (
  <svg {...base} width={12} height={12} viewBox="0 0 12 12" strokeWidth={1.4} {...p}>
    <path d="M6.75 1.25 2.5 6.75h3.25l-.5 4 4.25-5.5H6.25l.5-4Z" />
  </svg>
);

export const IconFolder = (p: P) => (
  <svg {...base} {...p}>
    <path d="M1.75 4.25A1.5 1.5 0 0 1 3.25 2.75h2.6l1.4 1.5h5.5a1.5 1.5 0 0 1 1.5 1.5v6a1.5 1.5 0 0 1-1.5 1.5h-9.5a1.5 1.5 0 0 1-1.5-1.5v-7.5Z" />
  </svg>
);

export const IconFile = (p: P) => (
  <svg {...base} {...p}>
    <path d="M4 1.75h5.25L12.5 5v8.5a.75.75 0 0 1-.75.75h-7.5a.75.75 0 0 1-.75-.75V2.5a.75.75 0 0 1 .5-.75Z" />
    <path d="M9 1.75V5.25h3.5" />
  </svg>
);

export const IconTrash = (p: P) => (
  <svg {...base} {...p}>
    <path d="M2.75 4.25h10.5M6.25 2.25h3.5M4.25 4.25l.6 8.6a1 1 0 0 0 1 .9h4.3a1 1 0 0 0 1-.9l.6-8.6" />
  </svg>
);

export const IconMove = (p: P) => (
  <svg {...base} {...p}>
    <path d="M2.25 8h10M9.25 4.75 12.5 8l-3.25 3.25" />
  </svg>
);

export const IconCopy = (p: P) => (
  <svg {...base} {...p}>
    <rect x="5.25" y="5.25" width="8" height="8" rx="1.5" />
    <path d="M10.75 5.25V3.75a1.5 1.5 0 0 0-1.5-1.5h-5.5a1.5 1.5 0 0 0-1.5 1.5v5.5a1.5 1.5 0 0 0 1.5 1.5h1.5" />
  </svg>
);

export const IconRename = (p: P) => (
  <svg {...base} {...p}>
    <path d="M9.75 2.75 13.25 6.25 6 13.5H2.5V10L9.75 2.75Z" />
  </svg>
);

export const IconReveal = (p: P) => (
  <svg {...base} {...p}>
    <path d="M9.25 2.25h4.5v4.5M13.5 2.5 7.75 8.25M11.75 9.25v3.5a1 1 0 0 1-1 1h-7.5a1 1 0 0 1-1-1v-7.5a1 1 0 0 1 1-1h3.5" />
  </svg>
);

export const IconMore = (p: P) => (
  <svg {...base} {...p}>
    <circle cx="3.5" cy="8" r="0.6" fill="currentColor" />
    <circle cx="8" cy="8" r="0.6" fill="currentColor" />
    <circle cx="12.5" cy="8" r="0.6" fill="currentColor" />
  </svg>
);

export const IconFiles = (p: P) => (
  <svg {...base} {...p}>
    <path d="M2.75 3.75h10.5M2.75 8h10.5M2.75 12.25h6.5" />
  </svg>
);

export const IconActivity = (p: P) => (
  <svg {...base} {...p}>
    <path d="M1.75 8h2.75l1.75-4.5 3.5 9 1.75-4.5h2.75" />
  </svg>
);
