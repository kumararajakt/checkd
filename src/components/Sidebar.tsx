type SidebarProps = {
  expanded: boolean;
  onToggle: () => void;
};

const NAV_ITEMS = [
  { label: "Todos", icon: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" },
  { label: "Done", icon: "M5 13l4 4L19 7" },
];

const SETTINGS = {
  label: "Settings",
  icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
};

type NavItem = {
  label: string;
  icon: string;
};

function NavButton({ item, expanded }: { item: NavItem; expanded: boolean }) {
  return (
    <button
      type="button"
      title={item.label}
      className={`flex items-center gap-3 rounded-md px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 ${
        expanded ? "justify-start" : "justify-center"
      }`}
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        strokeWidth={1.5}
        stroke="currentColor"
        className="size-5 shrink-0"
      >
        <path strokeLinecap="round" strokeLinejoin="round" d={item.icon} />
      </svg>
      {expanded && <span className="truncate">{item.label}</span>}
    </button>
  );
}

export default function Sidebar({ expanded, onToggle }: SidebarProps) {
  return (
    <aside
      className={`relative flex h-full shrink-0 flex-col border-r border-gray-200 bg-white transition-[width] duration-200 ease-in-out ${
        expanded ? "w-1/3" : "w-16"
      }`}
    >
      <div className="flex h-12 items-center gap-2 border-b border-gray-200 px-4">
        <img src="/checkd.svg" alt="checkd" className="size-6 shrink-0" />
        {expanded && <span className="truncate font-semibold">checkd</span>}
      </div>

      <nav className="flex flex-1 flex-col gap-1 p-2">
        {NAV_ITEMS.map((item) => (
          <NavButton key={item.label} item={item} expanded={expanded} />
        ))}
      </nav>

      <div className="border-t border-gray-200 p-2">
        <NavButton item={SETTINGS} expanded={expanded} />
      </div>

      {/* Expand/collapse toggle pinned to the sidebar's outer edge */}
      <button
        type="button"
        onClick={onToggle}
        aria-label={expanded ? "Collapse sidebar" : "Expand sidebar"}
        className="absolute top-16 -right-3 flex size-6 items-center justify-center rounded-full border border-gray-200 bg-white text-gray-500 shadow-sm hover:bg-gray-50"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={2}
          stroke="currentColor"
          className={`size-4 transition-transform duration-200 ${expanded ? "" : "rotate-180"}`}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M15.75 19.5L8.25 12l7.5-7.5" />
        </svg>
      </button>
    </aside>
  );
}
