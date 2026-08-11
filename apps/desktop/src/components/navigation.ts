export interface NavItem {
  to: string;
  label: string;
  iconType: "statistics" | "settings" | "about";
}

export const NAV_ITEMS: NavItem[] = [
  { to: "/", label: "Statistics", iconType: "statistics" },
  { to: "/settings", label: "Settings", iconType: "settings" },
  { to: "/about", label: "About", iconType: "about" },
];
