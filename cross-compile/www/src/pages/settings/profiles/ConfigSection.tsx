import * as React from 'react';

import { Button } from '@/components/ui/button';

export function ConfigSection({
  title,
  icon,
  active,
  token,
  details,
  onAdd,
  onEdit,
  onRemove,
  testId,
}: {
  readonly title: string;
  readonly icon: React.ReactNode;
  readonly active: boolean;
  readonly token?: string;
  readonly details?: string;
  readonly onAdd?: () => void;
  readonly onEdit?: () => void;
  readonly onRemove?: () => void;
  readonly testId?: string;
}) {
  // Active tiles offer edit/remove; inactive ones only offer add.
  const renderActions = () => {
    if (active) {
      return (
        <>
          {onEdit && (
            <Button
              variant="link"
              className="h-auto p-0 text-[11px] text-[#0a84ff]"
              onClick={onEdit}
              data-testid={testId ? `${testId}-edit-button` : undefined}
            >
              Edit
            </Button>
          )}
          {onRemove && (
            <Button
              variant="link"
              className="h-auto p-0 text-[11px] text-[#a1a1a6] hover:text-[#ff375f]"
              onClick={onRemove}
              data-testid={testId ? `${testId}-remove-button` : undefined}
            >
              Remove
            </Button>
          )}
        </>
      );
    }

    if (onAdd) {
      return (
        <Button
          variant="link"
          className="h-auto p-0 text-[11px] text-[#0a84ff]"
          onClick={onAdd}
          data-testid={testId ? `${testId}-add-button` : undefined}
        >
          Add
        </Button>
      );
    }

    return null;
  };

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
      <div className="mt-[8px] flex gap-[8px]">{renderActions()}</div>
    </div>
  );
}
