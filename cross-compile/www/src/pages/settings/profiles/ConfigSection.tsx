import * as React from 'react';

import { Button } from '@/components/ui/button';

export function ConfigSection({
  title,
  icon,
  active,
  token,
  details,
  onEdit,
  testId,
}: {
  readonly title: string;
  readonly icon: React.ReactNode;
  readonly active: boolean;
  readonly token?: string;
  readonly details?: string;
  readonly onEdit?: () => void;
  readonly testId?: string;
}) {
  return (
    <div
      className={`rounded-[8px] border p-[12px] ${active ? 'border-[#3a3a3c] bg-[#1c1c1e]' : 'border-[#3a3a3c]/50 bg-[#1c1c1e]/50 opacity-50'}`}
      data-testid={testId}
    >
      <div className="mb-[8px] flex items-center gap-[8px]">
        {icon}
        <span className="text-[13px] font-medium text-white">{title}</span>
      </div>
      {active ? (
        <div className="space-y-[2px]">
          {details && <div className="truncate text-[12px] text-[#a1a1a6]">{details}</div>}
          <div className="truncate font-mono text-[10px] text-[#636366]">{token}</div>
        </div>
      ) : (
        <div className="text-[12px] text-[#636366] italic">Not configured</div>
      )}
      <Button
        variant="link"
        className={`mt-[8px] h-auto p-0 text-[11px] ${active ? 'text-[#0a84ff]' : 'text-[#a1a1a6]'}`}
        onClick={onEdit}
        disabled={!onEdit}
        data-testid={testId ? `${testId}-edit-button` : undefined}
      >
        {active ? 'Edit' : 'Add (Coming Soon)'}
      </Button>
    </div>
  );
}
