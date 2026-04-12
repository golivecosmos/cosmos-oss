import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ClusterInsight {
  cluster_id: number;
  llm_name: string;
  llm_insight: string;
}

export interface BriefingNotice {
  notice_text: string;
  notice_type: string;
}

export interface BriefingResult {
  cluster_insights: ClusterInsight[];
  notices: BriefingNotice[];
  used_llm: boolean;
}

export type BriefingPhase =
  | "idle"
  | "enriching_cluster"
  | "generating_notices"
  | "complete";

export interface UseBriefingReturn {
  insights: ClusterInsight[];
  notices: BriefingNotice[];
  isLoading: boolean;
  usedLlm: boolean;
  phase: BriefingPhase;
  enrichingClusterName: string | null;
  generateBriefing: () => Promise<void>;
}

export function useBriefing(): UseBriefingReturn {
  const [insights, setInsights] = useState<ClusterInsight[]>([]);
  const [notices, setNotices] = useState<BriefingNotice[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [usedLlm, setUsedLlm] = useState(false);
  const [phase, setPhase] = useState<BriefingPhase>("idle");
  const [enrichingClusterName, setEnrichingClusterName] = useState<string | null>(null);
  const isMounted = useRef(true);

  const generateBriefing = useCallback(async () => {
    setIsLoading(true);
    setPhase("enriching_cluster");
    try {
      const result = await invoke<BriefingResult>("generate_briefing");
      if (!isMounted.current) return;
      setInsights(result.cluster_insights);
      setNotices(result.notices);
      setUsedLlm(result.used_llm);
      setPhase("complete");
    } catch (e) {
      if (!isMounted.current) return;
      console.error("Failed to generate briefing:", e);
      setPhase("complete");
    } finally {
      if (isMounted.current) {
        setIsLoading(false);
        setEnrichingClusterName(null);
      }
    }
  }, []);

  // Listen for progress events
  useEffect(() => {
    isMounted.current = true;
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;

    const setup = async () => {
      unlistenProgress = await listen("briefing_progress", (event: any) => {
        if (!isMounted.current) return;
        const payload = event?.payload;
        if (payload?.stage === "enriching_cluster") {
          setPhase("enriching_cluster");
          setEnrichingClusterName(payload.cluster_name || null);
        }
      });

      unlistenComplete = await listen("briefing_complete", () => {
        if (!isMounted.current) return;
        setPhase("complete");
      });
    };

    setup();

    return () => {
      isMounted.current = false;
      if (unlistenProgress) unlistenProgress();
      if (unlistenComplete) unlistenComplete();
    };
  }, []);

  return {
    insights,
    notices,
    isLoading,
    usedLlm,
    phase,
    enrichingClusterName,
    generateBriefing,
  };
}
