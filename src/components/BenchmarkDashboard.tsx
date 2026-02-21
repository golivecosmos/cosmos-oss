import React, { useState } from 'react'
import { Button } from './ui/button'
import { Card, CardContent, CardHeader, CardTitle } from './ui/card'
import { Badge } from './ui/badge'
import { invoke } from '@tauri-apps/api/tauri'
import { Clock, Database, Zap, Target, TrendingUp, TrendingDown, Minus } from 'lucide-react'

interface BenchmarkResult {
  query: string;
  backend: string;
  query_time_ms: number;
  result_count: number;
  error: string | null;
  index_size_mb: number | null;
  results: any[];
}

interface BenchmarkResponse {
  sqlite_result: BenchmarkResult;
  performance_comparison: {
    time_difference_ms: number;
    time_difference_percent: number;
    result_overlap_percent: number;
    total_unique_results: number;
    total_sqlite_results: number;
    unique_to_sqlite: number;
    correlation: number | null;
  };
}

interface BenchmarkDashboardProps {
  isOpen: boolean;
  onClose: () => void;
}

export function BenchmarkDashboard({ isOpen, onClose }: BenchmarkDashboardProps) {
  const [result, setResult] = useState<BenchmarkResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');

  if (!isOpen) return null

  const runBenchmark = async () => {
    if (!query.trim()) {
      setError('Please enter a search query');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await invoke('run_benchmark', { query });
      setResult(response as BenchmarkResponse);
    } catch (e) {
      setError(e as string);
    } finally {
      setIsLoading(false);
    }
  };

  const formatTime = (ms: number) => {
    return `${ms.toFixed(2)}ms`;
  };

  const formatBytes = (mb?: number) => {
    if (!mb) return 'N/A'
    if (mb < 1) return `${(mb * 1024).toFixed(1)}KB`
    if (mb < 1024) return `${mb.toFixed(1)}MB`
    return `${(mb / 1024).toFixed(2)}GB`
  }

  const getPerformanceIcon = (faster: boolean) => {
    return faster ? <TrendingUp className="h-4 w-4 text-green-500" /> : <TrendingDown className="h-4 w-4 text-red-500" />
  }

  const testQueries = [
    "blue ocean waves",
    "mountain landscape sunset", 
    "city at night",
    "forest trees",
    "modern architecture",
    "vintage car",
    "flower garden",
    "snow mountain"
  ]

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-7xl h-full max-h-[90vh] flex flex-col">
        <div className="p-6 border-b border-gray-200 flex justify-between items-center">
          <div>
            <h2 className="text-2xl font-bold text-gray-900">Search Performance Benchmark</h2>
            <p className="text-gray-600">Test SQLite vector search performance</p>
          </div>
          <Button onClick={onClose} variant="outline">Close</Button>
        </div>

        <div className="flex-1 overflow-hidden flex flex-col">
          {/* Test Controls */}
          <div className="p-6 border-b border-gray-200">
            <div className="flex flex-wrap gap-2 mb-4">
              <span className="text-sm font-medium text-gray-700">Quick Tests:</span>
              {testQueries.map((query) => (
                <Button
                  key={query}
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    setQuery(query);
                    runBenchmark();
                  }}
                  disabled={isLoading}
                  className="text-xs"
                >
                  {query}
                </Button>
              ))}
            </div>
            
            <div className="flex gap-2">
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Enter search query..."
                className="flex-1 px-3 py-2 border border-gray-300 rounded-md"
              />
              <Button 
                onClick={runBenchmark}
                disabled={isLoading}
              >
                {isLoading ? 'Running...' : 'Run Benchmark'}
              </Button>
            </div>
          </div>

          {/* Results */}
          <div className="flex-1 overflow-y-auto p-6">
            {isLoading && (
              <div className="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
                <div className="flex items-center gap-2">
                  <Clock className="h-4 w-4 text-blue-600 animate-spin" />
                  <span className="text-blue-800">Running benchmark for: "{query}"</span>
                </div>
              </div>
            )}

            {error && (
              <div className="mb-4 p-3 bg-red-100 text-red-700 rounded">
                {error}
              </div>
            )}

            {result && (
              <div className="space-y-6">
                <div className="grid grid-cols-1 gap-4">
                  {/* SQLite Results */}
                  <div className="p-4 border rounded">
                    <h3 className="text-lg font-semibold mb-2">SQLite</h3>
                    <div className="grid grid-cols-2 gap-2 text-sm">
                      <div className="text-gray-600">Query Time:</div>
                      <div>{formatTime(result.sqlite_result.query_time_ms)}</div>
                      <div className="text-gray-600">Results Found:</div>
                      <div>{result.sqlite_result.result_count}</div>
                      <div className="text-gray-600">Index Size:</div>
                      <div>
                        {result.sqlite_result.index_size_mb
                          ? `${result.sqlite_result.index_size_mb.toFixed(2)} MB`
                          : 'Unknown'}
                      </div>
                    </div>
                    {result.sqlite_result.error && (
                      <div className="mt-2 p-2 bg-red-50 rounded">
                        <div className="text-sm text-red-800">{result.sqlite_result.error}</div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
} 