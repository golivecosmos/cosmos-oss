interface CacheEntry<T> {
  results: T[];
  timestamp: number;
}

export class SearchCache<T = any> {
  private cache = new Map<string, CacheEntry<T>>();
  private readonly ttl: number;
  private readonly maxSize: number;

  constructor(ttl: number = 5 * 60 * 1000, maxSize: number = 50) {
    this.ttl = ttl;
    this.maxSize = maxSize;
  }

  get(key: string): T[] | null {
    console.log("🔍 Cache check for:", key, "Cache size:", this.cache.size);
    
    const entry = this.cache.get(key);
    if (!entry) {
      console.log("❌ No cache entry found for:", key);
      return null;
    }

    if (Date.now() - entry.timestamp > this.ttl) {
      console.log("⏰ Cache entry expired for:", key);
      this.cache.delete(key);
      return null;
    }

    console.log("🚀 Using cached search results for:", key, "Results count:", entry.results.length);
    return entry.results;
  }

  set(key: string, results: T[]): void {
    console.log("💾 Caching results for:", key, "Results count:", results.length);
    
    this.cache.set(key, {
      results: [...results], // Clone to avoid mutations
      timestamp: Date.now(),
    });

    // Clean up old entries if cache is too large
    if (this.cache.size > this.maxSize) {
      const oldestKey = Array.from(this.cache.keys())[0];
      console.log("🗑️ Removing oldest cache entry:", oldestKey);
      this.cache.delete(oldestKey);
    }

    console.log("📊 Cache size after update:", this.cache.size);
  }

  clear(): void {
    console.log("🧹 Clearing search cache");
    this.cache.clear();
  }

  size(): number {
    return this.cache.size;
  }

  has(key: string): boolean {
    const entry = this.cache.get(key);
    return entry !== undefined && Date.now() - entry.timestamp <= this.ttl;
  }
}