import { createSlice, PayloadAction, createAsyncThunk } from '@reduxjs/toolkit';
import { createDeviceManagementService } from '../../services/deviceManagementService';
import type { CameraConfig, DeviceInfo, SystemCapabilities } from '../../types';

export interface DeviceState {
  deviceInfo: DeviceInfo | null;
  systemCapabilities: SystemCapabilities | null;
  deviceStatus: {
    online: boolean;
    lastChecked: Date | null;
    deviceInfo: string;
    capabilities: string;
    services: {
      device: boolean;
      media: boolean;
      ptz: boolean;
      imaging: boolean;
    };
  } | null;
  isLoading: boolean;
  error: string | null;
}

const initialState: DeviceState = {
  deviceInfo: null,
  systemCapabilities: null,
  deviceStatus: null,
  isLoading: false,
  error: null,
};

// Async thunks for device management operations
export const getDeviceInformation = createAsyncThunk(
  'device/getDeviceInformation',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.getDeviceInformation();
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to get device information');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const getSystemCapabilities = createAsyncThunk(
  'device/getSystemCapabilities',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.getSystemCapabilities();
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to get system capabilities');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const getDeviceStatus = createAsyncThunk(
  'device/getDeviceStatus',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.getDeviceStatus();
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to get device status');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const setSystemDateAndTime = createAsyncThunk(
  'device/setSystemDateAndTime',
  async ({ config, dateTime, timeZone }: { config: CameraConfig; dateTime: Date; timeZone?: string }, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.setSystemDateAndTime(dateTime, timeZone);
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to set system date and time');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const rebootDevice = createAsyncThunk(
  'device/rebootDevice',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.rebootDevice();
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to reboot device');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const factoryReset = createAsyncThunk(
  'device/factoryReset',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.factoryReset();
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to factory reset device');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const updateDeviceConfiguration = createAsyncThunk(
  'device/updateDeviceConfiguration',
  async ({ config, newConfig }: { config: CameraConfig; newConfig: Partial<CameraConfig> }, { rejectWithValue }) => {
    try {
      const service = createDeviceManagementService(config);
      const result = await service.updateDeviceConfiguration(newConfig);
      
      if (result.success) {
        return result.data;
      } else {
        return rejectWithValue(result.error || 'Failed to update device configuration');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

const deviceSlice = createSlice({
  name: 'device',
  initialState,
  reducers: {
    clearError: (state) => {
      state.error = null;
    },
    setLoading: (state, action: PayloadAction<boolean>) => {
      state.isLoading = action.payload;
    },
    clearDeviceData: (state) => {
      state.deviceInfo = null;
      state.systemCapabilities = null;
      state.deviceStatus = null;
      state.error = null;
    }
  },
  extraReducers: (builder) => {
    builder
      // Get Device Information
      .addCase(getDeviceInformation.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(getDeviceInformation.fulfilled, (state, action) => {
        state.isLoading = false;
        state.deviceInfo = action.payload;
        state.error = null;
      })
      .addCase(getDeviceInformation.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      
      // Get System Capabilities
      .addCase(getSystemCapabilities.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(getSystemCapabilities.fulfilled, (state, action) => {
        state.isLoading = false;
        state.systemCapabilities = action.payload;
        state.error = null;
      })
      .addCase(getSystemCapabilities.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      
      // Get Device Status
      .addCase(getDeviceStatus.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(getDeviceStatus.fulfilled, (state, action) => {
        state.isLoading = false;
        state.deviceStatus = action.payload;
        state.error = null;
      })
      .addCase(getDeviceStatus.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      
      // Set System Date and Time
      .addCase(setSystemDateAndTime.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(setSystemDateAndTime.fulfilled, (state) => {
        state.isLoading = false;
        state.error = null;
      })
      .addCase(setSystemDateAndTime.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      
      // Reboot Device
      .addCase(rebootDevice.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(rebootDevice.fulfilled, (state) => {
        state.isLoading = false;
        state.error = null;
      })
      .addCase(rebootDevice.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      
      // Factory Reset
      .addCase(factoryReset.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(factoryReset.fulfilled, (state) => {
        state.isLoading = false;
        state.error = null;
      })
      .addCase(factoryReset.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      })
      
      // Update Device Configuration
      .addCase(updateDeviceConfiguration.pending, (state) => {
        state.isLoading = true;
        state.error = null;
      })
      .addCase(updateDeviceConfiguration.fulfilled, (state) => {
        state.isLoading = false;
        state.error = null;
      })
      .addCase(updateDeviceConfiguration.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
      });
  }
});

export const { clearError, setLoading, clearDeviceData } = deviceSlice.actions;
export default deviceSlice.reducer;
