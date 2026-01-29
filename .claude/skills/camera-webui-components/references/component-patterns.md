# Camera WebUI Component Patterns Reference

## Form Pattern: Multi-Step Settings

```typescript
interface StepData {
  identification: DeviceIdentification;
  network: NetworkSettings;
  advanced: AdvancedSettings;
}

interface DeviceSettingsWizardProps {
  onSubmit: (data: StepData) => Promise<void>;
  initialData?: StepData;
}

export function DeviceSettingsWizard({
  onSubmit,
  initialData,
}: DeviceSettingsWizardProps) {
  const [step, setStep] = useState(0);
  const [data, setData] = useState<StepData>(initialData || getDefaultData());
  const [isLoading, setIsLoading] = useState(false);

  const handleNext = () => {
    if (validateStep(step, data)) {
      setStep(step + 1);
    }
  };

  const handleSubmit = async () => {
    try {
      setIsLoading(true);
      await onSubmit(data);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="space-y-4">
      {/* Step indicator */}
      <div className="flex gap-2">
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            className={`h-2 flex-1 rounded-full ${
              i <= step ? 'bg-primary' : 'bg-border'
            }`}
          />
        ))}
      </div>

      {/* Step content */}
      <div>
        {step === 0 && (
          <IdentificationStep
            data={data.identification}
            onChange={(id) => setData({ ...data, identification: id })}
          />
        )}
        {step === 1 && (
          <NetworkStep
            data={data.network}
            onChange={(net) => setData({ ...data, network: net })}
          />
        )}
        {step === 2 && (
          <AdvancedStep
            data={data.advanced}
            onChange={(adv) => setData({ ...data, advanced: adv })}
          />
        )}
      </div>

      {/* Navigation */}
      <div className="flex justify-between pt-4">
        <Button
          variant="outline"
          onClick={() => setStep(step - 1)}
          disabled={step === 0 || isLoading}
        >
          Previous
        </Button>
        <Button
          onClick={step === 2 ? handleSubmit : handleNext}
          disabled={isLoading}
          className="bg-primary hover:bg-primary-hover"
        >
          {step === 2 ? 'Save' : 'Next'}
        </Button>
      </div>
    </div>
  );
}
```

## Table with Actions Pattern

```typescript
interface Device {
  id: string;
  name: string;
  ip: string;
  status: 'online' | 'offline';
}

interface DeviceTableProps {
  devices: Device[];
  onEdit: (id: string) => void;
  onDelete: (id: string) => Promise<void>;
}

export function DeviceTable({ devices, onEdit, onDelete }: DeviceTableProps) {
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const handleDelete = async (id: string) => {
    try {
      setDeletingId(id);
      await onDelete(id);
    } finally {
      setDeletingId(null);
    }
  };

  return (
    <div className="overflow-x-auto border border-border rounded-lg">
      <table className="w-full text-sm">
        <thead className="bg-background border-b border-border">
          <tr>
            <th className="px-4 py-3 text-left font-medium text-foreground">
              Name
            </th>
            <th className="px-4 py-3 text-left font-medium text-foreground">
              IP Address
            </th>
            <th className="px-4 py-3 text-left font-medium text-foreground">
              Status
            </th>
            <th className="px-4 py-3 text-right font-medium text-foreground">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          {devices.map((device) => (
            <tr
              key={device.id}
              className="border-t border-border hover:bg-card/50"
              data-testid={`device-row-${device.id}`}
            >
              <td className="px-4 py-3 text-foreground">{device.name}</td>
              <td className="px-4 py-3 text-muted-foreground">{device.ip}</td>
              <td className="px-4 py-3">
                <StatusBadge
                  status={device.status === 'online' ? 'active' : 'inactive'}
                  label={device.status}
                />
              </td>
              <td className="px-4 py-3 text-right space-x-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onEdit(device.id)}
                  data-testid={`edit-device-${device.id}`}
                >
                  Edit
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-destructive hover:text-destructive"
                  onClick={() => handleDelete(device.id)}
                  disabled={deletingId === device.id}
                  data-testid={`delete-device-${device.id}`}
                >
                  {deletingId === device.id ? 'Deleting...' : 'Delete'}
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

## Modal Form Pattern

```typescript
interface EditUserModalProps {
  isOpen: boolean;
  user?: User;
  onClose: () => void;
  onSave: (user: User) => Promise<void>;
}

export function EditUserModal({
  isOpen,
  user,
  onClose,
  onSave,
}: EditUserModalProps) {
  const [formData, setFormData] = useState<User>(user || getDefaultUser());
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(false);

  const handleSave = async () => {
    // Validate
    const newErrors = validateUser(formData);
    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }

    try {
      setIsLoading(true);
      await onSave(formData);
      onClose();
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent data-testid="edit-user-modal" className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {user ? 'Edit User' : 'Add New User'}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <FormInput
            label="Username"
            id="username"
            testId="edit-user-username-input"
            value={formData.username}
            onChange={(username) => setFormData({ ...formData, username })}
            error={errors.username}
            disabled={isLoading || !!user} // Don't allow changing username on edit
          />

          <FormInput
            label="Password"
            id="password"
            testId="edit-user-password-input"
            type="password"
            value={formData.password}
            onChange={(password) => setFormData({ ...formData, password })}
            error={errors.password}
            disabled={isLoading}
            placeholder={user ? 'Leave blank to keep current' : undefined}
          />

          <FormSelect
            label="Role"
            id="role"
            testId="edit-user-role-select"
            options={[
              { value: 'admin', label: 'Administrator' },
              { value: 'operator', label: 'Operator' },
              { value: 'user', label: 'User' },
            ]}
            value={formData.role}
            onChange={(role) => setFormData({ ...formData, role })}
            disabled={isLoading}
          />
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={onClose}
            disabled={isLoading}
            data-testid="edit-user-modal-cancel"
          >
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={isLoading}
            className="bg-primary hover:bg-primary-hover"
            data-testid="edit-user-modal-save"
          >
            {isLoading ? 'Saving...' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

## Tabs Component Pattern

```typescript
interface SettingsTabsProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export function SettingsTabs({ activeTab, onTabChange }: SettingsTabsProps) {
  const tabs = [
    { id: 'general', label: 'General', testId: 'tab-general' },
    { id: 'network', label: 'Network', testId: 'tab-network' },
    { id: 'advanced', label: 'Advanced', testId: 'tab-advanced' },
  ];

  return (
    <div>
      <div className="flex border-b border-border">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            data-testid={tab.testId}
            onClick={() => onTabChange(tab.id)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="pt-4">
        {activeTab === 'general' && <GeneralSettings />}
        {activeTab === 'network' && <NetworkSettings />}
        {activeTab === 'advanced' && <AdvancedSettings />}
      </div>
    </div>
  );
}
```

## Empty State Pattern

```typescript
interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  testId?: string;
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  testId,
}: EmptyStateProps) {
  return (
    <div
      data-testid={testId}
      className="flex flex-col items-center justify-center py-12 px-4"
    >
      {icon && <div className="mb-4 text-4xl text-muted-foreground">{icon}</div>}
      <h3 className="text-lg font-semibold text-foreground mb-2">{title}</h3>
      <p className="text-sm text-muted-foreground text-center max-w-sm mb-4">
        {description}
      </p>
      {action && (
        <Button
          onClick={action.onClick}
          className="bg-primary hover:bg-primary-hover"
          data-testid={`${testId}-action`}
        >
          {action.label}
        </Button>
      )}
    </div>
  );
}
```

## Alert/Toast Pattern

```typescript
type AlertLevel = 'info' | 'success' | 'warning' | 'error';

interface AlertProps {
  level: AlertLevel;
  title: string;
  message: string;
  onDismiss?: () => void;
  dismissible?: boolean;
  testId?: string;
}

export function Alert({
  level,
  title,
  message,
  onDismiss,
  dismissible = true,
  testId,
}: AlertProps) {
  const styles: Record<AlertLevel, string> = {
    info: 'bg-blue-500/10 border-blue-500/30 text-blue-500',
    success: 'bg-success/10 border-success/30 text-success',
    warning: 'bg-warning/10 border-warning/30 text-warning',
    error: 'bg-destructive/10 border-destructive/30 text-destructive',
  };

  return (
    <div
      data-testid={testId}
      className={`p-4 border rounded-lg ${styles[level]}`}
      role="alert"
    >
      <div className="flex items-start gap-3">
        <div className="flex-1">
          <h4 className="font-semibold">{title}</h4>
          <p className="text-sm mt-1 opacity-90">{message}</p>
        </div>
        {dismissible && (
          <button
            onClick={onDismiss}
            className="text-lg leading-none hover:opacity-70"
            aria-label="Dismiss"
          >
            ×
          </button>
        )}
      </div>
    </div>
  );
}
```

## Loading Skeleton Pattern

```typescript
export function CardSkeleton() {
  return (
    <Card className="bg-card border-border">
      <CardHeader>
        <div className="h-6 bg-border/50 rounded w-1/3 animate-pulse" />
      </CardHeader>
      <CardContent className="space-y-3">
        {[1, 2, 3].map((i) => (
          <div key={i} className="h-10 bg-border/50 rounded animate-pulse" />
        ))}
      </CardContent>
    </Card>
  );
}

export function CardGridSkeleton() {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {[1, 2, 3].map((i) => (
        <CardSkeleton key={i} />
      ))}
    </div>
  );
}
```
