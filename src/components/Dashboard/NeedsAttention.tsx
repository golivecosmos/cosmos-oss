import React from "react";
import { AlertCircle, Link2, TrendingUp, Search } from "lucide-react";
import type { BriefingNotice } from "../../hooks/useBriefing";

interface NeedsAttentionProps {
  notices: BriefingNotice[];
}

const NOTICE_ICONS: Record<string, React.ReactNode> = {
  missing: <AlertCircle className="w-3.5 h-3.5 text-amber-500 shrink-0 mt-0.5" />,
  connection: <Link2 className="w-3.5 h-3.5 text-blue-500 shrink-0 mt-0.5" />,
  growth: <TrendingUp className="w-3.5 h-3.5 text-emerald-500 shrink-0 mt-0.5" />,
  observation: <Search className="w-3.5 h-3.5 text-purple-500 shrink-0 mt-0.5" />,
  stat: <Search className="w-3.5 h-3.5 text-zinc-500 shrink-0 mt-0.5" />,
};

export const NeedsAttention: React.FC<NeedsAttentionProps> = ({ notices }) => {
  if (notices.length === 0) return null;

  return (
    <div className="rounded-xl border border-border p-5">
      <h2 className="text-sm font-semibold mb-3 flex items-center gap-2">
        <span className="text-amber-500">💡</span>
        Needs Attention
      </h2>
      <div className="space-y-2.5">
        {notices.map((notice, i) => (
          <div key={i} className="flex items-start gap-2.5">
            {NOTICE_ICONS[notice.notice_type] || NOTICE_ICONS.observation}
            <p className="text-sm text-muted-foreground leading-relaxed">
              {notice.notice_text}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
};
