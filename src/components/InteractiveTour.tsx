import React, { useState, useEffect, useRef } from 'react'
import { 
  FolderOpen, 
  Search, 
  Image, 
  FileText, 
  ArrowRight, 
  ArrowLeft, 
  X, 
  CheckCircle, 
  Target,
  MousePointer,
  Upload,
  Eye,
  Zap,
  Brain,
  Play,
  Pause,
  SkipForward
} from 'lucide-react'
import { Button } from "./ui/button"
import { Progress } from "./ui/progress"
import { motion, AnimatePresence } from 'framer-motion'
import { listen } from '@tauri-apps/api/event'

interface TourStep {
  id: string
  title: string
  description: string
  target: string // CSS selector for the element to highlight
  position: 'top' | 'bottom' | 'left' | 'right' | 'center'
  action?: 'click' | 'hover' | 'upload' | 'wait' | 'interactive'
  actionDescription?: string
  isInteractive?: boolean
  waitForEvent?: string // Event to wait for before proceeding
  customContent?: React.ReactNode
}

interface InteractiveTourProps {
  onComplete: () => void
  onDismiss: () => void
  onIndexFile?: (path: string) => Promise<void>
  isVisible: boolean
}

const tourSteps: TourStep[] = [
  {
    id: 'welcome',
    title: '🎉 Welcome to Your AI-Powered File Explorer!',
    description: 'Let\'s take a quick interactive tour to show you the magic. This will only take 2 minutes and you\'ll learn how to supercharge your file management with full control over your privacy!',
    target: 'body',
    position: 'center',
    action: 'wait'
  },
  {
    id: 'home-nav-button',
    title: '📁 Your File System',
    description: 'This is your home directory navigation button. Click it to expand and browse all your files and folders just like in Finder or Explorer. But here\'s the magic: you can right-click any folder to index it with AI!',
    target: '[data-tour="home-nav-button"]',
    position: 'right',
    action: 'click',
    actionDescription: 'Click to expand your home directory',
    isInteractive: true,
    waitForEvent: 'home-nav-expanded'
  },
  {
    id: 'index-action',
    title: '🧠 AI Indexing Magic',
    description: 'Perfect! Now you can browse your file tree and right-click on any folder to see the "Index Directory" option. This will analyze all images and documents in that folder, making them instantly searchable by content! You have full control over what gets indexed. Feel free to try it now, or click Next to continue.',
    target: '[data-tour="home-nav-expanded"]',
    position: 'right',
    action: 'interactive',
    actionDescription: 'Right-click on a folder in the file tree to index it',
    isInteractive: true,
    waitForEvent: 'indexing-started'
  },
  {
    id: 'collections',
    title: '📚 AI Library',
    description: 'Once files are indexed, they appear here! This shows all your AI-analyzed content. The number shows how many files have been processed and are ready for intelligent search.',
    target: '[data-tour="ai-library-button"]',
    position: 'right',
    action: 'click',
    actionDescription: 'Click on "AI Library" to see your processed content',
    isInteractive: true,
    waitForEvent: 'ai-library-clicked'
  },
  {
    id: 'ai-library-content',
    title: '📚 Your AI Library',
    description: 'Perfect! Here you can see all your indexed files. The AI has analyzed each file and made them searchable by content, colors, objects, and more. You can browse, search, or preview any file.',
    target: '[data-tour="preview-area"]',
    position: 'left',
    action: 'wait',
    actionDescription: 'Your indexed files are displayed here'
  },
  {
    id: 'search-bar',
    title: '🔍 Intelligent Search',
    description: 'This is where the magic happens! Search for anything - text inside images, color palettes, or even describe what you\'re looking for in natural language. Try it out, or click Next to continue.',
    target: '[data-tour="search-bar"]',
    position: 'bottom',
    action: 'interactive',
    actionDescription: 'Try searching for something like "warm golden hour" or "woman dancing outside"',
    isInteractive: true,
    waitForEvent: 'search-performed'
  },
  {
    id: 'search-results',
    title: '📋 Search Results',
    description: 'Perfect! Your search results appear here in the preview area. You can see thumbnails, file names, and metadata. Click on any result to preview it or use the search bar to refine your query.',
    target: '[data-tour="preview-area"]',
    position: 'left',
    action: 'wait',
    actionDescription: 'Your search results are displayed here'
  },
  {
    id: 'visual-search',
    title: '🖼️ Visual Search',
    description: 'Click the image icon to upload a photo and find similar images! This uses AI to understand visual content and find matches. Give it a try!',
    target: '[data-tour="visual-search"]',
    position: 'bottom',
    action: 'interactive',
    actionDescription: 'Click the image icon and upload a photo to try visual search',
    isInteractive: true,
    waitForEvent: 'visual-search-performed'
  },
  {
    id: 'reference-image',
    title: '🖼️ Reference Image & Results',
    description: 'Here is your reference image! Below, you’ll see all visually similar results. You can click on any result to preview it. Try clicking on a result to see a preview.',
    target: '[data-tour="reference-image"]',
    position: 'bottom',
    action: 'wait',
  },
  {
    id: 'preview-area',
    title: '👀 Smart Preview',
    description: 'Your search results and file previews appear here. You can see thumbnails, metadata, and even preview content without opening files!',
    target: '[data-tour="preview-area"]',
    position: 'left',
    action: 'wait'
  },
  {
    id: 'complete',
    title: '🚀 You\'re Ready to Go!',
    description: 'That\'s it! You now know how to index folders with AI, search intelligently, and find files faster than ever. Start by indexing a folder with your photos or documents!',
    target: 'body',
    position: 'center',
    action: 'wait'
  }
]

// Spotlight overlay component
const SpotlightOverlay = ({ target, children }: { target: string; children: React.ReactNode }) => {
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null)

  useEffect(() => {
    const updateTargetRect = () => {
      if (target === 'body') {
        setTargetRect(null)
        return
      }
      
      const element = document.querySelector(target)
      if (element) {
        const rect = element.getBoundingClientRect()
        setTargetRect(rect)
        console.log(`Tour: Found target element for "${target}"`, element, 'with rect:', rect)
      } else {
        console.warn(`Tour: Could not find target element for "${target}"`)
        // Debug: log all elements with data-tour attributes
        const allTourElements = document.querySelectorAll('[data-tour]')
        console.log('Available tour elements:', Array.from(allTourElements).map(el => ({
          tour: el.getAttribute('data-tour'),
          classes: el.className,
          tagName: el.tagName
        })))
        
        // For expanded NavButton targets, retry after a delay
        if (target === '[data-tour="home-nav-expanded"]') {
          console.log('Retrying to find home-nav-expanded in 200ms...')
          setTimeout(updateTargetRect, 200)
        }
      }
    }

    updateTargetRect()
    window.addEventListener('resize', updateTargetRect)
    window.addEventListener('scroll', updateTargetRect)

    return () => {
      window.removeEventListener('resize', updateTargetRect)
      window.removeEventListener('scroll', updateTargetRect)
    }
  }, [target])

  if (target === 'body' || !targetRect) {
    return (
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center">
        {children}
      </div>
    )
  }

  const spotlightStyle = {
    clipPath: `polygon(
      0% 0%, 
      0% 100%, 
      ${targetRect.left}px 100%, 
      ${targetRect.left}px ${targetRect.top}px, 
      ${targetRect.right}px ${targetRect.top}px, 
      ${targetRect.right}px ${targetRect.bottom}px, 
      ${targetRect.left}px ${targetRect.bottom}px, 
      ${targetRect.left}px 100%, 
      100% 100%, 
      100% 0%
    )`
  }

  return (
    <>
      {/* Spotlight overlay */}
      <div 
        className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 pointer-events-none"
        style={spotlightStyle}
      />
      
      {/* Highlighted element border */}
      <div
        className="fixed z-50 pointer-events-none border-4 border-blue-400 rounded-lg shadow-lg"
        style={{
          left: targetRect.left - 4,
          top: targetRect.top - 4,
          width: targetRect.width + 8,
          height: targetRect.height + 8,
          boxShadow: '0 0 0 4px rgba(59, 130, 246, 0.3), 0 0 20px rgba(59, 130, 246, 0.5)'
        }}
      />
      
      {/* Tour content */}
      <div className="fixed inset-0 z-50 pointer-events-none flex items-center justify-center">
        {children}
      </div>
    </>
  )
}

// Tour step card component
const TourStepCard = ({ 
  step, 
  currentStep, 
  totalSteps, 
  onNext, 
  onPrev, 
  onSkip, 
  onComplete,
  isWaitingForAction,
  actionCompleted 
}: {
  step: TourStep
  currentStep: number
  totalSteps: number
  onNext: () => void
  onPrev: () => void
  onSkip: () => void
  onComplete: () => void
  isWaitingForAction: boolean
  actionCompleted: boolean
}) => {
  const getPositionClasses = () => {
    switch (step.position) {
      case 'top':
        return 'items-start pt-20'
      case 'bottom':
        return 'items-end pb-20'
      case 'left':
        return 'items-center justify-start pl-20'
      case 'right':
        return 'items-center justify-end pr-20'
      default:
        return 'items-center justify-center'
    }
  }

  const isLastStep = currentStep === totalSteps - 1
  const isFirstStep = currentStep === 0

  return (
    <div className={`w-full h-full flex ${getPositionClasses()} pointer-events-none`}>
      <motion.div
        initial={{ opacity: 0, scale: 0.9, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: -20 }}
        className="dark:bg-darkBg bg-white rounded-2xl shadow-2xl border dark:border-darkBgHighlight border-gray-200 max-w-md w-full mx-4 pointer-events-auto"
      >
        {/* Header */}
        <div className="p-6 border-b dark:border-darkBgHighlight border-gray-100">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center">
              <div className="w-10 h-10 bg-gradient-to-r dark:from-customBlue from-blue-500 dark:to-blueShadow to-indigo-600 rounded-full flex items-center justify-center mr-3">
                <Target className="w-5 h-5 text-white" />
              </div>
              <div>
                <div className="text-sm dark:text-customGray text-gray-500">Step {currentStep + 1} of {totalSteps}</div>
                <Progress value={(currentStep + 1) / totalSteps * 100} className="w-32 h-2 mt-1" />
              </div>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={onSkip}
              className="dark:text-customGray text-gray-400 dark:hover:text-customRed hover:text-gray-600"
            >
              <X className="w-4 h-4" />
            </Button>
          </div>
          
          <h3 className="text-xl font-bold dark:text-white text-gray-900 mb-2">{step.title}</h3>
          <p className="dark:text-customBlue text-gray-600 leading-relaxed">{step.description}</p>
        </div>

        {/* Action area */}
        {step.action && step.action !== 'wait' && (
          <div className="p-4 dark:bg-darkBgMid dark:border-darkBgHighlight bg-gray-50 border-b border-gray-100">
            <div className="flex items-center">
              {step.isInteractive ? (
                <div className="flex items-center ">
                  {isWaitingForAction ? (
                    <>
                      <div className="w-6 h-6 dark:bg-customBlue bg-blue-500 rounded-full flex items-center justify-center mr-3 animate-pulse">
                        <MousePointer className="w-3 h-3 text-white" />
                      </div>
                      <span className="text-sm font-medium dark:text-customBlue text-blue-700">
                        {step.actionDescription}
                      </span>
                    </>
                  ) : actionCompleted ? (
                    <>
                      <div className="w-6 h-6 dark:bg-customGreen bg-green-500 rounded-full flex items-center justify-center mr-3">
                        <CheckCircle className="w-3 h-3 text-white" />
                      </div>
                      <span className="text-sm dark:text-customBlue font-medium text-green-700">
                        Great! Action completed
                      </span>
                    </>
                  ) : (
                    <>
                      <div className="w-6 h-6 dark:bg-customBlue bg-gray-400 rounded-full flex items-center justify-center mr-3">
                        <Play className="w-3 h-3 text-white" />
                      </div>
                      <span className="text-sm dark:text-customBlue text-gray-600">
                        {step.actionDescription}
                      </span>
                    </>
                  )}
                </div>
              ) : (
                <div className="flex items-center">
                  <div className="w-6 h-6 dark:bg-customBlue bg-blue-500 rounded-full flex items-center justify-center mr-3">
                    <Eye className="w-3 h-3 text-white" />
                  </div>
                  <span className="text-sm dark:text-customBlue text-gray-600">
                    {step.actionDescription}
                  </span>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Custom content */}
        {step.customContent && (
          <div className="p-4 border-b border-gray-100">
            {step.customContent}
          </div>
        )}

        {/* Footer */}
        <div className="p-6 flex items-center justify-between">
          <Button
            variant="outline"
            onClick={onPrev}
            disabled={isFirstStep}
            className="flex items-center"
          >
            <ArrowLeft className="w-4 h-4 mr-2" />
            Previous
          </Button>

          <div className="flex items-center space-x-2">
            <Button
              variant="ghost"
              onClick={onSkip}
              className="dark:text-customGray text-gray-500 dark:hover:text-customBlue hover:text-gray-700"
            >
              Skip Tour
            </Button>
            
            {isLastStep ? (
              <Button
                onClick={onComplete}
                className="bg-gradient-to-r dark:from-customGreen dark:hover:from-greenShadow dark:hover:to-greenShadow from-green-500 dark:to-greenShadow to-emerald-600 hover:from-green-600 hover:to-emerald-700 text-white"
              >
                <CheckCircle className="w-4 h-4 mr-2" />
                Complete Tour
              </Button>
            ) : (
              <Button
                onClick={onNext}
                className="bg-gradient-to-r dark:from-customBlue from-blue-500 dark:to-blueShadow to-indigo-600 dark:hover:from-blueShadow dark:hover:to-darkBgHighlight hover:from-blue-600 hover:to-indigo-700 text-white"
              >
                Next
                <ArrowRight className="w-4 h-4 ml-2" />
              </Button>
            )}
          </div>
        </div>
      </motion.div>
    </div>
  )
}

export function InteractiveTour({ onComplete, onDismiss, onIndexFile, isVisible }: InteractiveTourProps) {
  const [currentStep, setCurrentStep] = useState(0)
  const [isWaitingForAction, setIsWaitingForAction] = useState(false)
  const [actionCompleted, setActionCompleted] = useState(false)
  const [hasStartedIndexing, setHasStartedIndexing] = useState(false)

  const currentTourStep = tourSteps[currentStep]

  // Listen for events that indicate user actions
  useEffect(() => {
    const handleIndexingStarted = () => {
      if (currentTourStep.waitForEvent === 'indexing-started') {
        setActionCompleted(true)
        setIsWaitingForAction(false)
        setHasStartedIndexing(true)
        // Auto-advance after a short delay
        setTimeout(() => {
          handleNext()
        }, 2000)
      }
    }

    const handleSearchPerformed = () => {
      if (currentTourStep.waitForEvent === 'search-performed') {
        setActionCompleted(true)
        setIsWaitingForAction(false)
        // Auto-advance immediately to show search results
        setTimeout(() => {
          handleNext()
        }, 500) // Reduced from 2000ms to 500ms for faster response
      }
    }

    const handleAILibraryClicked = () => {
      if (currentTourStep.waitForEvent === 'ai-library-clicked') {
        setActionCompleted(true)
        setIsWaitingForAction(false)
        // Auto-advance to show the AI Library content
        setTimeout(() => {
          handleNext()
        }, 500)
      }
    }

    const handleVisualSearchPerformed = () => {
      if (currentTourStep.waitForEvent === 'visual-search-performed') {
        setActionCompleted(true)
        setIsWaitingForAction(false)
        // Auto-advance to the new reference image/results step
        setTimeout(() => {
          setCurrentStep((prev) => prev + 1)
        }, 2000)
      }
    }

    const handleHomeNavExpanded = () => {
      if (currentTourStep.waitForEvent === 'home-nav-expanded') {
        setActionCompleted(true)
        setIsWaitingForAction(false)
        // Auto-advance to step 3
        setTimeout(() => {
          handleNext()
        }, 500)
      }
    }

    // Listen for bulk index progress events
    const handleBulkIndexProgress = (event: any) => {
      if (event.detail && event.detail.status === 'starting') {
        handleIndexingStarted()
      }
    }

    // Listen for Tauri events from the backend
    const handleJobsBatchCreated = () => {
      if (currentTourStep.waitForEvent === 'indexing-started') {
        setActionCompleted(true)
        setIsWaitingForAction(false)
        setHasStartedIndexing(true)
        // Auto-advance after a short delay
        setTimeout(() => {
          handleNext()
        }, 2000)
      }
    }

    window.addEventListener('indexing-started', handleIndexingStarted)
    window.addEventListener('search-performed', handleSearchPerformed)
    window.addEventListener('ai-library-clicked', handleAILibraryClicked)
    window.addEventListener('visual-search-performed', handleVisualSearchPerformed)
    window.addEventListener('home-nav-expanded', handleHomeNavExpanded)
    window.addEventListener('bulk_index_progress', handleBulkIndexProgress)

    // Listen for Tauri events
    let unlistenJobsBatchCreated: any = null
    
    const setupTauriListeners = async () => {
      try {
        unlistenJobsBatchCreated = await listen('jobs_batch_created', handleJobsBatchCreated)
      } catch (error) {
        console.error('Failed to setup Tauri event listeners for tour:', error)
      }
    }
    
    setupTauriListeners()

    return () => {
      window.removeEventListener('indexing-started', handleIndexingStarted)
      window.removeEventListener('search-performed', handleSearchPerformed)
      window.removeEventListener('ai-library-clicked', handleAILibraryClicked)
      window.removeEventListener('visual-search-performed', handleVisualSearchPerformed)
      window.removeEventListener('home-nav-expanded', handleHomeNavExpanded)
      window.removeEventListener('bulk_index_progress', handleBulkIndexProgress)
      
      if (unlistenJobsBatchCreated) {
        unlistenJobsBatchCreated()
      }
    }
  }, [currentTourStep])

  // Handle step changes
  useEffect(() => {
    const step = tourSteps[currentStep]
    
    if (step.isInteractive && step.action !== 'wait') {
      setIsWaitingForAction(true)
      setActionCompleted(false)
    } else {
      setIsWaitingForAction(false)
      setActionCompleted(false)
    }

    // Clean up the expanded container attribute when leaving step 3
    if (step.id !== 'index-action') {
      const expandedContainer = document.querySelector('[data-tour="home-nav-expanded"]')
      if (expandedContainer) {
        expandedContainer.removeAttribute('data-tour')
        console.log('Removed data-tour="home-nav-expanded" from container')
      }
    }

    // Add tour data attributes to elements and handle NavButton expansion
    const addTourAttributes = () => {
      // Search bar
      const searchBar = document.querySelector('form[class*="search"], input[placeholder*="Search"]')?.closest('form')
      if (searchBar) {
        searchBar.setAttribute('data-tour', 'search-bar')
      }

      // Visual search button - it already has data-tour="visual-search"
      const visualSearchBtn = document.querySelector('[data-tour="visual-search"]')
      if (visualSearchBtn) {
        // Button already has the correct attribute
        console.log('Found visual search button for tour')
      } else {
        // Fallback: try to find by button with Image icon
        const allButtons = document.querySelectorAll('button')
        for (const btn of allButtons) {
          const imageIcon = btn.querySelector('svg[class*="lucide-image"]')
          if (imageIcon) {
            btn.setAttribute('data-tour', 'visual-search')
            console.log('Found visual search button via fallback')
            break
          }
        }
      }

      // Preview area - look for the main content area that contains the file grid
      const previewArea = document.querySelector('[class*="preview-area"], .preview-area, [class*="grid-view"], .flex-1.flex.flex-col.h-full.bg-gray-50')
      if (previewArea) {
        previewArea.setAttribute('data-tour', 'preview-area')
        console.log('Found preview area for tour:', previewArea)
      } else {
        // Fallback: try to find the main content area by looking for the PreviewContainer
        const previewContainer = document.querySelector('[class*="PreviewContainer"], [class*="preview-container"]')
        if (previewContainer) {
          const parentContainer = previewContainer.closest('.flex-1.flex.flex-col.h-full.bg-gray-50') || 
                                  previewContainer.closest('.flex-1') ||
                                  previewContainer.parentElement
          if (parentContainer) {
            parentContainer.setAttribute('data-tour', 'preview-area')
            console.log('Found preview area via fallback:', parentContainer)
          }
        }
      }

      // Reference image panel
      const referenceImagePanel = document.querySelector('[class*="reference-image"], [class*="ReferenceImagePanel"], [data-testid="reference-image"]')
      if (referenceImagePanel) {
        referenceImagePanel.setAttribute('data-tour', 'reference-image')
      }

      // Handle NavButton expansion for tour steps
      const currentTarget = currentTourStep.target
      
      // For step 3 (index-action), ensure NavButton is expanded and add the expanded container attribute
      if (currentTourStep.id === 'index-action') {
        const homeNavButton = document.querySelector('[data-tour="home-nav-button"]')
        if (homeNavButton) {
          const navButtonContainer = homeNavButton.closest('[data-nav-button]')?.parentElement
          if (navButtonContainer) {
            console.log('Found NavButton container for step 3:', navButtonContainer)
            
            const isExpanded = navButtonContainer.getAttribute('data-expanded') === 'true'
            console.log('Is expanded:', isExpanded)
            
            if (!isExpanded) {
              // Trigger click to expand
              (homeNavButton as HTMLElement).click()
              console.log('Expanded home NavButton for tour (step 3)')
              
              // Wait a bit for the expansion animation to complete
              setTimeout(() => {
                navButtonContainer.setAttribute('data-tour', 'home-nav-expanded')
                console.log('Added data-tour="home-nav-expanded" to container:', navButtonContainer)
              }, 350) // Wait for animation to complete
            } else {
              // Already expanded, just add the attribute
              navButtonContainer.setAttribute('data-tour', 'home-nav-expanded')
              console.log('Added data-tour="home-nav-expanded" to already expanded container:', navButtonContainer)
            }
          }
        }
      }
    }

    // Add attributes after a short delay to ensure DOM is ready
    // For step 3 (index-action), wait longer for animation to complete
    const delay = currentTourStep.id === 'index-action' ? 500 : 200
    setTimeout(addTourAttributes, delay)
  }, [currentStep])

  const handleNext = () => {
    if (currentStep < tourSteps.length - 1) {
      setCurrentStep(currentStep + 1)
    } else {
      onComplete()
    }
  }

  const handlePrev = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1)
    }
  }



  const handleComplete = () => {
    onComplete()
  }

  const handleSkip = () => {
    onDismiss()
  }

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      // Clean up any remaining tour attributes
      const expandedContainer = document.querySelector('[data-tour="home-nav-expanded"]')
      if (expandedContainer) {
        expandedContainer.removeAttribute('data-tour')
      }
    }
  }, [])

  if (!isVisible) {
    return null
  }

  return (
    <AnimatePresence>
      <SpotlightOverlay target={currentTourStep.target}>
        <TourStepCard
          step={currentTourStep}
          currentStep={currentStep}
          totalSteps={tourSteps.length}
          onNext={handleNext}
          onPrev={handlePrev}
          onSkip={handleSkip}
          onComplete={handleComplete}
          isWaitingForAction={isWaitingForAction}
          actionCompleted={actionCompleted}
        />
      </SpotlightOverlay>
    </AnimatePresence>
  )
} 
