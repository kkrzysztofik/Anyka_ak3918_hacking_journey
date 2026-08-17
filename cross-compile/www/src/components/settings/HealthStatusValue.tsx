import type { HealthBadgeTone } from '@/utils/identificationStatusCard';

const HEALTH_DOT_CLASS: Record<HealthBadgeTone, string> = {
  healthy: 'bg-green-500',
  degraded: 'bg-yellow-500',
  unreachable: 'bg-red-500',
  unknown: 'bg-[#6b6b6f]',
};

export function HealthStatusValue({
  label,
  tone,
  detail,
  testId = 'device-status-health',
}: Readonly<{ label: string; tone: HealthBadgeTone; detail?: string; testId?: string }>) {
  return (
    <div className="flex items-center gap-2" title={detail} data-testid={testId}>
      <div className={`size-2 rounded-full ${HEALTH_DOT_CLASS[tone]}`} />
      <span>{label}</span>
    </div>
  );
}
