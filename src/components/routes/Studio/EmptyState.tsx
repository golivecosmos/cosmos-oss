import React from "react";
import { Video, Brain, Download } from "lucide-react";

export const EmptyState: React.FC = () => {
  return (
    <div className="flex flex-col h-full">
      <div className="mb-8">
        <div className="w-32 h-32 bg-purple-100 dark:bg-purple-900/20 rounded-full flex items-center justify-center mx-auto mb-4">
          <Video className="w-16 h-16 text-purple-600 dark:text-purple-400" />
        </div>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white text-center mb-2">
          Create Your First Video
        </h2>
        <p className="text-lg text-gray-600 dark:text-gray-400 text-center mb-8">
          Write a prompt in the side panel to generate amazing videos with AI
        </p>
      </div>

      {/* Feature Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 w-full max-w-4xl mx-auto mb-8">
        <div className="p-6 bg-white dark:bg-darkBgHighlight rounded-xl border border-gray-200 dark:border-gray-700">
          <div className="w-12 h-12 bg-purple-100 dark:bg-purple-900/20 flex items-center justify-center mb-4 mx-auto rounded-full">
            <Brain className="w-6 h-6 text-purple-600 dark:text-purple-400" />
          </div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">Write Prompts</h3>
          <p className="text-gray-600 dark:text-gray-400">Describe your vision in detail for better results</p>
        </div>
        <div className="p-6 bg-white dark:bg-darkBgHighlight rounded-xl border border-gray-200 dark:border-gray-700">
          <div className="w-12 h-12 bg-blue-100 dark:bg-blue-900/20 flex items-center justify-center mb-4 mx-auto rounded-full">
            <Video className="w-6 h-6 text-blue-600 dark:text-blue-400" />
          </div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">Generate Videos</h3>
          <p className="text-gray-600 dark:text-gray-400">Create high-quality videos with Veo3 AI</p>
        </div>
        <div className="p-6 bg-white dark:bg-darkBgHighlight rounded-xl border border-gray-200 dark:border-gray-700">
          <div className="w-12 h-12 bg-green-100 dark:bg-green-900/20 flex items-center justify-center mb-4 mx-auto rounded-full">
            <Download className="w-6 h-6 text-green-600 dark:text-green-400" />
          </div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">Download & Share</h3>
          <p className="text-gray-600 dark:text-gray-400">Save your creations and share with others</p>
        </div>
      </div>
    </div>
  );
}; 
