import React from 'react'
import { Button } from "./ui/button"
import { Sparkles, RotateCcw } from 'lucide-react'

interface OnboardingTriggerProps {
  onTrigger: () => void
}

export function OnboardingTrigger({ onTrigger }: OnboardingTriggerProps) {
  return (
    <div className="fixed bottom-4 left-4 z-40">
      <Button
        onClick={onTrigger}
        variant="outline"
        size="sm"
        className="bg-white/90 backdrop-blur-sm border-gray-200 hover:bg-gray-50 shadow-lg"
      >
        <Sparkles className="w-4 h-4 mr-2" />
        Restart Welcome
        <RotateCcw className="w-3 h-3 ml-2" />
      </Button>
    </div>
  )
} 