import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

import { SearchCache } from "../utils/searchCache";

export type SearchType = "text" | "visual" | "tag" | "recent";
export type SemanticFileTypeFilter = "all" | "image" | "video" | "audio" | "document";

export interface SearchOptions {
  semanticFileTypeFilter?: SemanticFileTypeFilter;
}

export interface SearchState {
  query: string;
  results: any[];
  isSearching: boolean;
  type: SearchType;
  isSearchMode: boolean;
}

export interface UseSearchReturn {
  searchState: SearchState;
  handleSearch: (query: string, type: SearchType, options?: SearchOptions) => Promise<void>;
  clearSearch: () => void;
  refreshCurrentSearch: () => Promise<void>;
  clearCache: () => void;
  isSearching: boolean;
}

export const useSearch = (): UseSearchReturn => {
  const [searchState, setSearchState] = useState<SearchState>({
    query: "",
    results: [],
    isSearching: false,
    type: "text",
    isSearchMode: false
  });
  // Initialize cache with 5 minute TTL and max 50 entries
  const cacheRef = useRef(new SearchCache(5 * 60 * 1000, 50));
  const activeRequestIdRef = useRef(0);

  const handleSearch = useCallback(async (
    query: string,
    type: SearchType,
    options: SearchOptions = {}
  ): Promise<void> => {
    const requestId = ++activeRequestIdRef.current;

    try {
      // Handle empty query (clear search)
      if (!query && type !== "visual") {
        if (requestId !== activeRequestIdRef.current) return;
        setSearchState({
          query: "",
          results: [],
          isSearching: false,
          type: "text",
          isSearchMode: false
        });
        return;
      }

      // Set loading state
      setSearchState(prev => ({
        ...prev,
        query,
        type,
        isSearchMode: true,
        isSearching: true
      }));

      const semanticFilter = options.semanticFileTypeFilter || "all";
      const cacheKey = `${type}:${query}:${semanticFilter}`;
      // Check cache first for non-visual searches
      if (type !== "visual") {
        const cachedResults = cacheRef.current.get(cacheKey);
        if (cachedResults) {
          if (requestId !== activeRequestIdRef.current) return;
          setSearchState(prev => ({
            ...prev,
            results: cachedResults,
            isSearching: false
          }));
          return;
        }
      }

      // Sync drives to database before searching to ensure current drive status
      try {
        await invoke('sync_drives_to_database');
      } catch (syncError) {
        console.warn('Failed to sync drives before search:', syncError);
        // Continue with search even if sync fails
      }

      let searchResults: any[] = [];

      // Perform search based on type
      switch (type) {
        case "text":
          console.time("🔍 Semantic search");
          searchResults = await invoke<any[]>("search_semantic", {
            query,
            fileTypeFilter: semanticFilter === "all" ? null : semanticFilter,
          });
          console.timeEnd("🔍 Semantic search");

          if (Array.isArray(searchResults)) {
            cacheRef.current.set(cacheKey, searchResults);
          }
          break;

        case "visual":
          console.time("🖼️ Visual search");
          searchResults = await invoke<any[]>("search_visual", {
            imageData: query,
          });
          console.timeEnd("🖼️ Visual search");
          break;

        case "tag":
          // For tag search, get indexed files and filter by tag
          const indexedFilesForTag = await invoke<any[]>("get_indexed_files_grouped_paginated", {
            offset: 0,
            limit: 1000,
          });
          searchResults = indexedFilesForTag.filter(
            (file) => file.tags && file.tags.includes(query)
          );
          cacheRef.current.set(cacheKey, searchResults);
          break;

        case "recent":
          // For recent search, get indexed files and sort by date
          const indexedFilesForRecent = await invoke<any[]>("get_indexed_files_grouped_paginated", {
            offset: 0,
            limit: 1000,
          });
          searchResults = [...indexedFilesForRecent].sort(
            (a, b) =>
              new Date(b.updated_at || 0).getTime() -
              new Date(a.updated_at || 0).getTime()
          );
          cacheRef.current.set(cacheKey, searchResults);
          break;

        default:
          console.warn("Unknown search type:", type);
          searchResults = [];
      }

      // Update state with results
      if (requestId !== activeRequestIdRef.current) return;
      setSearchState(prev => ({
        ...prev,
        results: searchResults,
        isSearching: false
      }));

    } catch (error) {
      console.error("Search error:", error);
      if (requestId !== activeRequestIdRef.current) return;
      setSearchState(prev => ({
        ...prev,
        results: [],
        isSearching: false
      }));
    }
  }, []);

  const clearSearch = useCallback(() => {
    activeRequestIdRef.current += 1;
    setSearchState({
      query: "",
      results: [],
      isSearching: false,
      type: "text",
      isSearchMode: false
    });
    // Note: We don't clear the cache on navigation - only on explicit refresh
  }, []);

  const refreshCurrentSearch = useCallback(async () => {
    const { query, type, isSearchMode } = searchState;
    if (query && isSearchMode) {
      console.log("🔄 Refreshing current search:", query, type);
      // Clear cache for current search and re-run it
      const cacheKey = `${type}:${query}`;
      if (cacheRef.current.has(cacheKey)) {
        // Remove from cache to force fresh search
        cacheRef.current.clear(); // Clear entire cache to be safe
      }
      await handleSearch(query, type);
    }
  }, [searchState, handleSearch]);

  const clearCache = useCallback(() => {
    console.log("🧹 Clearing search cache");
    cacheRef.current.clear();
  }, []);

  return {
    searchState,
    handleSearch,
    clearSearch,
    refreshCurrentSearch,
    clearCache,
    isSearching: searchState.isSearching
  };
};
