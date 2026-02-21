import React from 'react';
import { Check, AlertTriangle, FileText, DollarSign, Phone, X, Clock, Globe, Tag } from 'lucide-react';
import { Button } from '../ui/button';

// Mock rights database - in production this would come from an API/backend
const MOCK_RIGHTS_DB = {
  "asset_id": "NBA_1994_FINALS_G5_MASTER",
  "title": "1994 NBA Finals Game 5 Highlight Reel",
  "source_licensor": "NBA Entertainment",
  "license_type": "Master Archive Agreement",
  "usage_rights": {
    "documentary": true,
    "promotional": true,
    "commercial": false,
    "broadcast": true
  },
  "restrictions": {
    "max_duration_promotional": "120 seconds",
    "attribution_required": "Courtesy NBA Entertainment",
    "territory_limitations": ["Asia Pacific excluded"]
  },
  "cost_structure": {
    "base_fee": "Covered under master agreement",
    "overage_fee": "$500 per 30-second increment above limit"
  },
  "expiration_date": "2027-06-30",
  "usage_history": [
    {"project": "30for30: Winning Time", "date": "2010-03-15", "duration": "45s"},
    {"project": "OJ Simpson Trailer", "date": "2024-10-15", "duration": "12s"}
  ],
  "contact_info": {
    "rights_contact": "sarah.chen@nba.com",
    "phone": "212-555-NBA1"
  },
  "technical_specs": {
    "file_format": "ProRes 422 HQ",
    "resolution": "1920x1080",
    "frame_rate": "29.97fps",
    "audio_channels": "Stereo + 5.1"
  }
};

interface VideoRightsSheetProps {
  videoPath: string;
  isOpen: boolean;
  onClose: () => void;
}

export function VideoRightsSheet({ videoPath, isOpen, onClose }: VideoRightsSheetProps) {
  // For now, we'll always return the mock data
  const rights = MOCK_RIGHTS_DB;
  
  // Calculate remaining promotional time
  const usedSeconds = rights.usage_history.reduce((total, usage) => {
    return total + parseInt(usage.duration);
  }, 0);
  const maxSeconds = parseInt(rights.restrictions.max_duration_promotional);
  const remainingSeconds = maxSeconds - usedSeconds;

  if (!isOpen) return null;

  return (
    <div className="fixed inset-y-0 right-0 w-[500px] bg-white shadow-xl border-l border-gray-200 flex flex-col z-50">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
        <div>
          <h2 className="text-lg font-semibold text-gray-900">Rights Information</h2>
          <p className="text-sm text-gray-500">{rights.title}</p>
        </div>
        <Button variant="ghost" size="icon" onClick={onClose}>
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {/* Key information */}
        <div className="space-y-4">
          <div className="flex items-start gap-3 p-4 bg-green-50 rounded-lg">
            <Check className="h-5 w-5 text-green-500 mt-0.5" />
            <div>
              <h3 className="font-medium text-green-900">Cleared for Use</h3>
              <p className="text-sm text-green-700">Promotional use up to {rights.restrictions.max_duration_promotional}</p>
            </div>
          </div>

          <div className="flex items-start gap-3 p-4 bg-amber-50 rounded-lg">
            <Clock className="h-5 w-5 text-amber-500 mt-0.5" />
            <div>
              <h3 className="font-medium text-amber-900">Usage Tracking</h3>
              <p className="text-sm text-amber-700">{usedSeconds} seconds used ({remainingSeconds} seconds remaining)</p>
              <div className="mt-2">
                <div className="h-2 bg-amber-200 rounded-full overflow-hidden">
                  <div 
                    className="h-full bg-amber-500 rounded-full"
                    style={{ width: `${(usedSeconds / maxSeconds) * 100}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Usage History */}
        <div className="mt-8">
          <h3 className="text-sm font-medium text-gray-900 mb-3">Usage History</h3>
          <div className="space-y-3">
            {rights.usage_history.map((usage, index) => (
              <div key={index} className="flex items-center justify-between text-sm p-3 bg-gray-50 rounded-lg">
                <div>
                  <p className="font-medium text-gray-900">{usage.project}</p>
                  <p className="text-gray-500">{usage.date}</p>
                </div>
                <span className="text-gray-700">{usage.duration}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Requirements and Restrictions */}
        <div className="mt-8">
          <h3 className="text-sm font-medium text-gray-900 mb-3">Requirements & Restrictions</h3>
          <div className="space-y-4">
            <div className="flex items-start gap-3">
              <FileText className="h-5 w-5 text-blue-500 mt-0.5" />
              <div>
                <h4 className="font-medium text-gray-900">Attribution Required</h4>
                <p className="text-sm text-gray-700">"{rights.restrictions.attribution_required}"</p>
              </div>
            </div>

            <div className="flex items-start gap-3">
              <Globe className="h-5 w-5 text-purple-500 mt-0.5" />
              <div>
                <h4 className="font-medium text-gray-900">Territory Limitations</h4>
                <p className="text-sm text-gray-700">{rights.restrictions.territory_limitations.join(", ")}</p>
              </div>
            </div>

            <div className="flex items-start gap-3">
              <DollarSign className="h-5 w-5 text-emerald-500 mt-0.5" />
              <div>
                <h4 className="font-medium text-gray-900">Cost Structure</h4>
                <p className="text-sm text-gray-700">{rights.cost_structure.base_fee}</p>
                <p className="text-sm text-gray-500">Overage: {rights.cost_structure.overage_fee}</p>
              </div>
            </div>
          </div>
        </div>

        {/* Technical Specs */}
        <div className="mt-8">
          <h3 className="text-sm font-medium text-gray-900 mb-3">Technical Specifications</h3>
          <div className="grid grid-cols-2 gap-4">
            {Object.entries(rights.technical_specs).map(([key, value]) => (
              <div key={key} className="text-sm">
                <p className="text-gray-500 capitalize">{key.replace(/_/g, ' ')}</p>
                <p className="font-medium text-gray-900">{value}</p>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="border-t border-gray-200 p-6">
        <div className="flex items-center gap-3 text-sm">
          <Phone className="h-4 w-4 text-gray-500" />
          <div>
            <p className="font-medium text-gray-900">{rights.contact_info.rights_contact}</p>
            <p className="text-gray-500">{rights.contact_info.phone}</p>
          </div>
        </div>
      </div>
    </div>
  );
} 