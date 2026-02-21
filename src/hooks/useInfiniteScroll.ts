import { useEffect, useCallback } from 'react';

export function useInfiniteScroll(
  targetRef: React.RefObject<HTMLElement>,
  onIntersect: () => void,
  options = {
    threshold: 0.1,
    rootMargin: '200px',
  }
) {
  const handleObserver = useCallback(
    (entries: IntersectionObserverEntry[]) => {
      const [entry] = entries;
      if (entry.isIntersecting) {
        onIntersect();
      }
    },
    [onIntersect]
  );

  useEffect(() => {
    const observer = new IntersectionObserver(handleObserver, options);
    const currentTarget = targetRef.current;

    if (currentTarget) {
      observer.observe(currentTarget);
    }

    return () => {
      if (currentTarget) {
        observer.unobserve(currentTarget);
      }
    };
  }, [targetRef, options.rootMargin, options.threshold, handleObserver]);
} 