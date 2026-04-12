import React, { useRef, useEffect, useCallback, useState } from "react";
import type { FileCluster, FilePosition2D } from "../../hooks/useClusters";

// 10 distinct cluster colors — hue-spaced for visual separation
const CLUSTER_COLORS = [
  "#6366f1", // indigo
  "#f43f5e", // rose
  "#10b981", // emerald
  "#f59e0b", // amber
  "#3b82f6", // blue
  "#ec4899", // pink
  "#14b8a6", // teal
  "#f97316", // orange
  "#8b5cf6", // violet
  "#06b6d4", // cyan
];

function clusterColor(id: number): string {
  return CLUSTER_COLORS[id % CLUSTER_COLORS.length];
}

interface ViewState {
  offsetX: number;
  offsetY: number;
  scale: number;
}

interface HoveredFile {
  position: FilePosition2D;
  screenX: number;
  screenY: number;
}

interface VisualMapProps {
  clusters: FileCluster[];
  positions: FilePosition2D[];
  selectedClusterId: number | null;
  onSelectCluster: (id: number | null) => void;
  onSelectFile: (fileId: string, filePath: string) => void;
  showLabels?: boolean;
}

export const VisualMap: React.FC<VisualMapProps> = ({
  clusters,
  positions,
  selectedClusterId,
  onSelectCluster,
  onSelectFile,
  showLabels = true,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<ViewState>({ offsetX: 0, offsetY: 0, scale: 1 });
  const isDragging = useRef(false);
  const dragStart = useRef({ x: 0, y: 0 });
  const [hovered, setHovered] = useState<HoveredFile | null>(null);
  const animFrameRef = useRef<number>(0);

  // Compute data bounds once
  const bounds = useRef({ minX: 0, maxX: 1, minY: 0, maxY: 1 });
  useEffect(() => {
    if (positions.length === 0) return;
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const p of positions) {
      if (p.x < minX) minX = p.x;
      if (p.x > maxX) maxX = p.x;
      if (p.y < minY) minY = p.y;
      if (p.y > maxY) maxY = p.y;
    }
    // Add 10% padding
    const padX = (maxX - minX) * 0.1 || 1;
    const padY = (maxY - minY) * 0.1 || 1;
    bounds.current = {
      minX: minX - padX,
      maxX: maxX + padX,
      minY: minY - padY,
      maxY: maxY + padY,
    };
    // Reset view to fit
    viewRef.current = { offsetX: 0, offsetY: 0, scale: 1 };
  }, [positions]);

  // Map data coords to canvas pixel coords
  const toScreen = useCallback(
    (dataX: number, dataY: number, canvas: HTMLCanvasElement) => {
      const { minX, maxX, minY, maxY } = bounds.current;
      const { offsetX, offsetY, scale } = viewRef.current;
      const w = canvas.width / window.devicePixelRatio;
      const h = canvas.height / window.devicePixelRatio;
      const normX = (dataX - minX) / Math.max(maxX - minX, 0.001);
      const normY = (dataY - minY) / Math.max(maxY - minY, 0.001);
      return {
        x: (normX * w + offsetX) * scale + (w * (1 - scale)) / 2,
        y: (normY * h + offsetY) * scale + (h * (1 - scale)) / 2,
      };
    },
    []
  );

  // Build cluster lookup
  const clusterMap = useRef<Map<number, FileCluster>>(new Map());
  useEffect(() => {
    const m = new Map<number, FileCluster>();
    for (const c of clusters) m.set(c.cluster_id, c);
    clusterMap.current = m;
  }, [clusters]);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width / dpr;
    const h = canvas.height / dpr;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.save();
    ctx.scale(dpr, dpr);

    const { scale } = viewRef.current;
    const dotRadius = Math.max(3, 5 * scale);

    // Draw dots
    for (const pos of positions) {
      const { x, y } = toScreen(pos.x, pos.y, canvas);
      // Viewport culling
      if (x < -dotRadius || x > w + dotRadius || y < -dotRadius || y > h + dotRadius) continue;

      const dimmed = selectedClusterId !== null && pos.cluster_id !== selectedClusterId;
      const color = clusterColor(pos.cluster_id);

      ctx.beginPath();
      ctx.arc(x, y, dotRadius, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.globalAlpha = dimmed ? 0.15 : 0.85;
      ctx.fill();
    }

    ctx.globalAlpha = 1;

    // Draw cluster labels
    if (showLabels && scale > 0.4) {
      ctx.font = `${Math.max(11, 13 * scale)}px system-ui, -apple-system, sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";

      for (const cluster of clusters) {
        const { x, y } = toScreen(cluster.position_x, cluster.position_y, canvas);
        if (x < -100 || x > w + 100 || y < -20 || y > h + 20) continue;

        const dimmed = selectedClusterId !== null && cluster.cluster_id !== selectedClusterId;
        if (dimmed) continue;

        const label = cluster.name;
        const metrics = ctx.measureText(label);
        const pad = 4;

        // Background pill
        ctx.fillStyle = "rgba(0,0,0,0.6)";
        ctx.beginPath();
        ctx.roundRect(
          x - metrics.width / 2 - pad,
          y - 8 - pad,
          metrics.width + pad * 2,
          16 + pad * 2,
          4
        );
        ctx.fill();

        // Text
        ctx.fillStyle = "#fff";
        ctx.fillText(label, x, y);
      }
    }

    ctx.restore();
  }, [positions, clusters, selectedClusterId, showLabels, toScreen]);

  // Resize observer
  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;

    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
      draw();
    };

    const observer = new ResizeObserver(resize);
    observer.observe(container);
    resize();

    return () => {
      observer.disconnect();
      cancelAnimationFrame(animFrameRef.current);
    };
  }, [draw]);

  // Redraw when data or view changes
  useEffect(() => {
    draw();
  }, [draw]);

  // Wheel zoom
  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      viewRef.current.scale = Math.min(20, Math.max(0.2, viewRef.current.scale * delta));
      cancelAnimationFrame(animFrameRef.current);
      animFrameRef.current = requestAnimationFrame(draw);
    },
    [draw]
  );

  // Pan
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    isDragging.current = true;
    dragStart.current = { x: e.clientX, y: e.clientY };
  }, []);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      if (isDragging.current) {
        const dx = e.clientX - dragStart.current.x;
        const dy = e.clientY - dragStart.current.y;
        viewRef.current.offsetX += dx / viewRef.current.scale;
        viewRef.current.offsetY += dy / viewRef.current.scale;
        dragStart.current = { x: e.clientX, y: e.clientY };
        cancelAnimationFrame(animFrameRef.current);
        animFrameRef.current = requestAnimationFrame(draw);
        return;
      }

      // Hit-test for hover tooltip
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const hitRadius = Math.max(6, 8 * viewRef.current.scale);

      let found: FilePosition2D | null = null;
      for (const pos of positions) {
        const { x, y } = toScreen(pos.x, pos.y, canvas);
        const dist = Math.sqrt((mx - x) ** 2 + (my - y) ** 2);
        if (dist <= hitRadius) {
          found = pos;
          break;
        }
      }

      if (found) {
        setHovered({ position: found, screenX: e.clientX, screenY: e.clientY });
        canvas.style.cursor = "pointer";
      } else {
        setHovered(null);
        canvas.style.cursor = isDragging.current ? "grabbing" : "grab";
      }
    },
    [positions, toScreen, draw]
  );

  const handleMouseUp = useCallback(() => {
    isDragging.current = false;
  }, []);

  // Click to select cluster or file
  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const hitRadius = Math.max(6, 8 * viewRef.current.scale);

      for (const pos of positions) {
        const { x, y } = toScreen(pos.x, pos.y, canvas);
        if (Math.sqrt((mx - x) ** 2 + (my - y) ** 2) <= hitRadius) {
          onSelectFile(pos.file_id, pos.file_path);
          return;
        }
      }

      // Clicked empty space — check cluster labels
      for (const cluster of clusters) {
        const { x, y } = toScreen(cluster.position_x, cluster.position_y, canvas);
        if (Math.abs(mx - x) < 50 && Math.abs(my - y) < 15) {
          onSelectCluster(
            selectedClusterId === cluster.cluster_id ? null : cluster.cluster_id
          );
          return;
        }
      }

      // Clicked empty space — deselect
      onSelectCluster(null);
    },
    [positions, clusters, selectedClusterId, toScreen, onSelectCluster, onSelectFile]
  );

  // Reset view (exposed via ref later if needed)
  const resetView = useCallback(() => {
    viewRef.current = { offsetX: 0, offsetY: 0, scale: 1 };
    draw();
  }, [draw]);

  // Keyboard: +/- zoom, Escape deselect
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "=" || e.key === "+") {
        viewRef.current.scale = Math.min(20, viewRef.current.scale * 1.2);
        draw();
      } else if (e.key === "-") {
        viewRef.current.scale = Math.max(0.2, viewRef.current.scale / 1.2);
        draw();
      } else if (e.key === "Escape") {
        onSelectCluster(null);
      } else if (e.key === "0") {
        resetView();
      }
    },
    [draw, onSelectCluster, resetView]
  );

  if (positions.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        <div className="text-center space-y-2">
          <p className="text-lg font-medium">No clusters yet</p>
          <p className="text-sm">Index some files to see your visual map</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="relative w-full h-full overflow-hidden" tabIndex={0} onKeyDown={handleKeyDown}>
      <canvas
        ref={canvasRef}
        className="w-full h-full cursor-grab"
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onClick={handleClick}
      />

      {/* Hover tooltip */}
      {hovered && (
        <div
          className="pointer-events-none fixed z-50 rounded-md bg-popover px-3 py-1.5 text-xs text-popover-foreground shadow-md border"
          style={{
            left: hovered.screenX + 12,
            top: hovered.screenY - 8,
          }}
        >
          <p className="font-medium truncate max-w-[240px]">
            {hovered.position.file_path.split("/").pop()}
          </p>
          <p className="text-muted-foreground truncate max-w-[240px]">
            {hovered.position.source_type} · Cluster {clusterMap.current.get(hovered.position.cluster_id)?.name || hovered.position.cluster_id}
          </p>
        </div>
      )}
    </div>
  );
};

export { CLUSTER_COLORS, clusterColor };
export type { VisualMapProps };
