import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { AlertCircle, FileText } from "lucide-react";

import { MediaFile } from "./types";
import { Button } from "../ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { cn } from "../../lib/utils";
import { getErrorMessage } from "../../utils/errorMessage";

interface DocumentPreviewProps {
  file: MediaFile;
}

interface FilePreviewPayload {
  content: string;
  truncated: boolean;
  total_bytes: number;
  read_bytes: number;
  encoding: string;
}

const DEFAULT_PREVIEW_BYTES = 256 * 1024;
const MAX_CSV_ROWS = 200;
const MAX_CSV_COLS = 40;

const TEXT_EXTENSIONS = new Set([
  "txt",
  "md",
  "markdown",
  "json",
  "js",
  "jsx",
  "ts",
  "tsx",
  "css",
  "html",
  "htm",
  "xml",
  "csv",
  "log",
  "yml",
  "yaml",
  "toml",
  "ini",
  "conf",
  "rtf",
  "py",
  "rb",
  "java",
  "cpp",
  "c",
  "h",
  "rs",
  "go",
  "php",
  "sql",
]);

function resolveFilesystemPath(path: string): string {
  if (path.startsWith("asset://localhost/")) {
    try {
      return decodeURIComponent(path.replace("asset://localhost", ""));
    } catch {
      return path.replace("asset://localhost", "");
    }
  }
  if (path.startsWith("file://")) {
    try {
      return decodeURIComponent(path.replace("file://", ""));
    } catch {
      return path.replace("file://", "");
    }
  }
  return path;
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function parseCsv(content: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let inQuotes = false;

  for (let i = 0; i < content.length; i += 1) {
    const ch = content[i];
    const next = content[i + 1];

    if (ch === '"') {
      if (inQuotes && next === '"') {
        cell += '"';
        i += 1;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }

    if (ch === "," && !inQuotes) {
      row.push(cell);
      cell = "";
      continue;
    }

    if ((ch === "\n" || ch === "\r") && !inQuotes) {
      if (ch === "\r" && next === "\n") {
        i += 1;
      }
      row.push(cell);
      rows.push(row);
      row = [];
      cell = "";
      continue;
    }

    cell += ch;
  }

  if (cell.length > 0 || row.length > 0) {
    row.push(cell);
    rows.push(row);
  }

  return rows;
}

export function DocumentPreview({ file }: DocumentPreviewProps) {
  const [preview, setPreview] = useState<FilePreviewPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [wrapLines, setWrapLines] = useState(true);
  const [showLineNumbers, setShowLineNumbers] = useState(false);
  const [loadFull, setLoadFull] = useState(false);
  const [htmlTab, setHtmlTab] = useState("rendered");

  const ext = file.name.split(".").pop()?.toLowerCase() || "";
  const isPdf = ext === "pdf";
  const isHtml = ext === "html" || ext === "htm";
  const isJson = ext === "json";
  const isCsv = ext === "csv";
  const isTextLike = TEXT_EXTENSIONS.has(ext);
  const fileSystemPath = resolveFilesystemPath(file.path);
  const pdfPreviewSrc = `${convertFileSrc(fileSystemPath)}#toolbar=0`;
  const sourceContent = preview?.content ?? "";

  useEffect(() => {
    setLoadFull(false);
    setHtmlTab("rendered");
  }, [file.path]);

  useEffect(() => {
    async function loadContent() {
      setIsLoading(true);
      setError(null);
      setPreview(null);

      try {
        if (!isTextLike || isPdf) {
          setIsLoading(false);
          return;
        }

        const payload = await invoke<FilePreviewPayload>("read_file_preview", {
          path: fileSystemPath,
          maxBytes: loadFull ? null : DEFAULT_PREVIEW_BYTES,
        });
        setPreview(payload);
      } catch (err) {
        setError(getErrorMessage(err));
        console.error("Error loading document:", err);
      } finally {
        setIsLoading(false);
      }
    }

    loadContent();
  }, [fileSystemPath, isTextLike, isPdf, loadFull]);

  const prettyJson = useMemo(() => {
    if (!isJson || !sourceContent) {
      return null;
    }
    try {
      return JSON.stringify(JSON.parse(sourceContent), null, 2);
    } catch {
      return null;
    }
  }, [isJson, sourceContent]);

  const textViewContent = prettyJson ?? sourceContent;
  const textLines = useMemo(() => textViewContent.split(/\r?\n/), [textViewContent]);

  const csvRows = useMemo(() => {
    if (!isCsv || !sourceContent) {
      return [];
    }
    return parseCsv(sourceContent);
  }, [isCsv, sourceContent]);
  const csvVisible = useMemo(
    () => csvRows.slice(0, MAX_CSV_ROWS).map((row) => row.slice(0, MAX_CSV_COLS)),
    [csvRows]
  );

  const renderToolbar = () => (
    <div className="flex items-center justify-between gap-2 border-b border-gray-200 dark:border-darkBgHighlight px-3 py-2">
      <div className="text-xs text-gray-500 dark:text-customGray">
        {preview ? `Showing ${formatBytes(preview.read_bytes)} of ${formatBytes(preview.total_bytes)}` : "No content"}
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="outline"
          className="h-7 px-2 text-xs"
          onClick={() => setWrapLines((value) => !value)}
        >
          {wrapLines ? "No Wrap" : "Wrap"}
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="h-7 px-2 text-xs"
          onClick={() => setShowLineNumbers((value) => !value)}
        >
          {showLineNumbers ? "Hide Lines" : "Line Numbers"}
        </Button>
        {preview?.truncated && !loadFull && (
          <Button
            size="sm"
            variant="default"
            className="h-7 px-2 text-xs"
            onClick={() => setLoadFull(true)}
          >
            Load Full File
          </Button>
        )}
      </div>
    </div>
  );

  const renderTextBody = (content: string) => (
    <div className="h-full overflow-auto">
      {showLineNumbers ? (
        <div className="font-mono text-sm">
          {textLines.map((line, index) => (
            <div key={index} className="grid grid-cols-[56px_1fr] border-b border-gray-100 dark:border-darkBgHighlight/40">
              <div className="select-none border-r border-gray-200 dark:border-darkBgHighlight px-2 py-1 text-right text-xs text-gray-400">
                {index + 1}
              </div>
              <div
                className={cn(
                  "px-3 py-1 text-gray-800 dark:text-gray-100",
                  wrapLines ? "whitespace-pre-wrap break-words" : "whitespace-pre"
                )}
              >
                {line.length > 0 ? line : " "}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <pre
          className={cn(
            "font-mono text-sm p-4 text-gray-800 dark:text-gray-100",
            wrapLines ? "whitespace-pre-wrap break-words" : "whitespace-pre"
          )}
        >
          {content}
        </pre>
      )}
    </div>
  );

  if (isLoading) {
    return (
      <div className="w-full h-full flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="w-full h-full flex flex-col items-center justify-center text-gray-500 gap-2">
        <AlertCircle className="h-8 w-8" />
        <p className="text-center max-w-xl">Failed to load document content: {error}</p>
      </div>
    );
  }

  if (isPdf) {
    return (
      <div className="w-full h-[calc(100vh-3rem)] flex flex-col overflow-hidden">
        <iframe
          src={pdfPreviewSrc}
          className="w-full h-full border-none"
          title={file.name}
        />
      </div>
    );
  }

  if (isHtml && preview) {
    return (
      <div className="w-full h-[calc(100vh-3rem)] flex flex-col overflow-hidden">
        <Tabs value={htmlTab} onValueChange={setHtmlTab} className="h-full flex flex-col">
          <div className="flex items-center justify-between border-b border-gray-200 dark:border-darkBgHighlight px-3 py-2">
            <TabsList className="h-8">
              <TabsTrigger className="text-xs" value="rendered">
                Rendered
              </TabsTrigger>
              <TabsTrigger className="text-xs" value="source">
                Source
              </TabsTrigger>
            </TabsList>
            <div className="text-xs text-gray-500 dark:text-customGray">
              {preview.truncated
                ? `Preview truncated to ${formatBytes(preview.read_bytes)}`
                : `${formatBytes(preview.total_bytes)} loaded`}
            </div>
          </div>
          <TabsContent value="rendered" className="m-0 flex-1 min-h-0">
            <iframe
              srcDoc={sourceContent}
              sandbox=""
              className="w-full h-full border-none bg-white"
              title={file.name}
            />
          </TabsContent>
          <TabsContent value="source" className="m-0 flex-1 min-h-0 flex flex-col">
            {renderToolbar()}
            {renderTextBody(sourceContent)}
          </TabsContent>
        </Tabs>
      </div>
    );
  }

  if (isCsv && preview) {
    return (
      <div className="w-full h-[calc(100vh-3rem)] flex flex-col overflow-hidden">
        <div className="flex items-center justify-between border-b border-gray-200 dark:border-darkBgHighlight px-3 py-2">
          <div className="text-xs text-gray-500 dark:text-customGray">
            {csvRows.length.toLocaleString()} rows
            {csvRows.length > MAX_CSV_ROWS ? ` (showing first ${MAX_CSV_ROWS})` : ""}
          </div>
          <div className="flex items-center gap-2">
            {preview.truncated && !loadFull && (
              <Button
                size="sm"
                variant="default"
                className="h-7 px-2 text-xs"
                onClick={() => setLoadFull(true)}
              >
                Load Full File
              </Button>
            )}
          </div>
        </div>
        <div className="h-full overflow-auto">
          {csvVisible.length === 0 ? (
            <div className="p-4 text-sm text-gray-500 dark:text-customGray">No rows found.</div>
          ) : (
            <table className="min-w-full border-collapse text-sm">
              <thead className="sticky top-0 z-10 bg-gray-100 dark:bg-darkBgHighlight">
                <tr>
                  {(csvVisible[0] || []).map((header, index) => (
                    <th
                      key={`header-${index}`}
                      className="border-b border-r border-gray-200 dark:border-darkBg px-3 py-2 text-left font-semibold text-gray-800 dark:text-gray-100"
                    >
                      {header || `Column ${index + 1}`}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {csvVisible.slice(1).map((row, rowIndex) => (
                  <tr key={`row-${rowIndex}`} className="odd:bg-white even:bg-gray-50 dark:odd:bg-darkBgMid dark:even:bg-darkBg">
                    {row.map((cell, colIndex) => (
                      <td
                        key={`cell-${rowIndex}-${colIndex}`}
                        className="border-b border-r border-gray-100 dark:border-darkBgHighlight px-3 py-2 align-top text-gray-700 dark:text-gray-200"
                      >
                        <div className={cn("max-w-[420px]", wrapLines ? "whitespace-pre-wrap break-words" : "truncate")}>
                          {cell}
                        </div>
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    );
  }

  if (isTextLike && preview) {
    return (
      <div className="w-full h-[calc(100vh-3rem)] flex flex-col overflow-hidden">
        {renderToolbar()}
        {renderTextBody(textViewContent)}
      </div>
    );
  }

  return (
    <div className="w-full h-[calc(100vh-3rem)] flex flex-col items-center justify-center text-gray-500">
      <FileText className="h-16 w-16 mb-2" />
      <p>Preview not available for this file type</p>
      <p className="text-sm mt-1">({file.name})</p>
    </div>
  );
}
