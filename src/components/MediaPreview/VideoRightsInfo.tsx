import React from 'react';
import { Check, AlertTriangle, FileText, DollarSign, Phone } from 'lucide-react';

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

interface VideoRightsInfoProps {
  videoPath: string; // In production, we'd use this to look up the actual rights
}

export function VideoRightsInfo({ videoPath }: VideoRightsInfoProps) {
  // For now, we'll always return the mock data
  const rights = MOCK_RIGHTS_DB;
  
  // Calculate remaining promotional time
  const usedSeconds = rights.usage_history.reduce((total, usage) => {
    return total + parseInt(usage.duration);
  }, 0);
  const maxSeconds = parseInt(rights.restrictions.max_duration_promotional);
  const remainingSeconds = maxSeconds - usedSeconds;

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4 space-y-2">
      <h3 className="font-medium text-gray-900 mb-3">Rights Information</h3>
      
      <div className="space-y-2 text-sm">
        {/* Promotional use status */}
        <div className="flex items-start gap-2">
          <Check className="h-4 w-4 text-green-500 mt-0.5 flex-shrink-0" />
          <span className="text-gray-700">
            Cleared for promotional use (up to {rights.restrictions.max_duration_promotional})
          </span>
        </div>

        {/* Usage tracking */}
        <div className="flex items-start gap-2">
          <AlertTriangle className="h-4 w-4 text-amber-500 mt-0.5 flex-shrink-0" />
          <span className="text-gray-700">
            {usedSeconds} seconds already used ({remainingSeconds} seconds remaining)
          </span>
        </div>

        {/* Attribution */}
        <div className="flex items-start gap-2">
          <FileText className="h-4 w-4 text-blue-500 mt-0.5 flex-shrink-0" />
          <span className="text-gray-700">
            Attribution required: "{rights.restrictions.attribution_required}"
          </span>
        </div>

        {/* Cost info */}
        <div className="flex items-start gap-2">
          <DollarSign className="h-4 w-4 text-emerald-500 mt-0.5 flex-shrink-0" />
          <span className="text-gray-700">
            {rights.cost_structure.base_fee}
          </span>
        </div>

        {/* Contact info */}
        <div className="flex items-start gap-2">
          <Phone className="h-4 w-4 text-purple-500 mt-0.5 flex-shrink-0" />
          <span className="text-gray-700">
            Rights contact: {rights.contact_info.rights_contact} ({rights.contact_info.phone})
          </span>
        </div>
      </div>

      {/* Additional details */}
      <div className="mt-4 pt-3 border-t border-gray-100">
        <div className="grid grid-cols-2 gap-2 text-xs">
          <div>
            <span className="text-gray-500">Source:</span>{" "}
            <span className="text-gray-700">{rights.source_licensor}</span>
          </div>
          <div>
            <span className="text-gray-500">License Type:</span>{" "}
            <span className="text-gray-700">{rights.license_type}</span>
          </div>
          <div>
            <span className="text-gray-500">Expires:</span>{" "}
            <span className="text-gray-700">{new Date(rights.expiration_date).toLocaleDateString()}</span>
          </div>
          <div>
            <span className="text-gray-500">Territory:</span>{" "}
            <span className="text-gray-700">{rights.restrictions.territory_limitations.join(", ")}</span>
          </div>
        </div>
      </div>
    </div>
  );
} 