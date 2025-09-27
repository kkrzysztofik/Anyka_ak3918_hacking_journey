import { createSlice, PayloadAction, createAsyncThunk } from '@reduxjs/toolkit';
import { createONVIFClient } from '../../services/onvifClient';
import type { ONVIFState, CameraConfig } from '../../types';

const initialState: ONVIFState = {
  status: 'checking',
  lastChecked: null,
  error: null,
  isInitialized: false,
};

// Async thunks for ONVIF operations
export const checkONVIFStatus = createAsyncThunk(
  'onvif/checkStatus',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.getDeviceInformation();
      
      if (result.success) {
        return { status: 'online' as const, timestamp: new Date() };
      } else {
        return rejectWithValue({
          status: 'offline' as const,
          error: result.error || 'Unknown error',
          timestamp: new Date(),
        });
      }
    } catch (error) {
      return rejectWithValue({
        status: 'offline' as const,
        error: error instanceof Error ? error.message : 'Unknown error',
        timestamp: new Date(),
      });
    }
  }
);

export const initializePTZ = createAsyncThunk(
  'onvif/initializePTZ',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.stopPTZ();
      
      if (result.success) {
        return { success: true };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const stopPTZ = createAsyncThunk(
  'onvif/stopPTZ',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.stopPTZ();
      
      if (result.success) {
        return { success: true };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const movePTZ = createAsyncThunk(
  'onvif/movePTZ',
  async ({ config, direction }: { config: CameraConfig; direction: string }, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.movePTZ(direction);
      
      if (result.success) {
        return { success: true, direction };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const getImagingSettings = createAsyncThunk(
  'onvif/getImagingSettings',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.getImagingSettings();
      
      if (result.success) {
        return { success: true, data: result.data };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const setImagingSettings = createAsyncThunk(
  'onvif/setImagingSettings',
  async ({ config, settings }: { config: CameraConfig; settings: any }, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.setImagingSettings(settings);
      
      if (result.success) {
        return { success: true, data: result.data };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const getPresets = createAsyncThunk(
  'onvif/getPresets',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.getPresets();
      
      if (result.success) {
        return { success: true, data: result.data };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const gotoPreset = createAsyncThunk(
  'onvif/gotoPreset',
  async ({ config, presetToken }: { config: CameraConfig; presetToken: string }, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.gotoPreset(presetToken);
      
      if (result.success) {
        return { success: true, data: result.data };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const setPreset = createAsyncThunk(
  'onvif/setPreset',
  async ({ config, presetName }: { config: CameraConfig; presetName: string }, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.setPreset(presetName);
      
      if (result.success) {
        return { success: true, data: result.data };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const getStreamUri = createAsyncThunk(
  'onvif/getStreamUri',
  async (config: CameraConfig, { rejectWithValue }) => {
    try {
      const client = createONVIFClient(config);
      const result = await client.getStreamUri();
      
      if (result.success) {
        return { success: true, data: result.data };
      } else {
        return rejectWithValue(result.error || 'Unknown error');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

const onvifSlice = createSlice({
  name: 'onvif',
  initialState,
  reducers: {
    setStatus: (state, action: PayloadAction<ONVIFState['status']>) => {
      state.status = action.payload;
    },
    setError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
    },
    clearError: (state) => {
      state.error = null;
    },
    setInitialized: (state, action: PayloadAction<boolean>) => {
      state.isInitialized = action.payload;
    },
  },
  extraReducers: (builder) => {
    builder
      // Check ONVIF Status
      .addCase(checkONVIFStatus.pending, (state) => {
        state.status = 'checking';
        state.error = null;
      })
      .addCase(checkONVIFStatus.fulfilled, (state, action) => {
        state.status = action.payload.status;
        state.lastChecked = action.payload.timestamp;
        state.error = null;
      })
      .addCase(checkONVIFStatus.rejected, (state, action) => {
        const payload = action.payload as any;
        state.status = payload?.status || 'offline';
        state.lastChecked = payload?.timestamp || new Date();
        state.error = payload?.error || 'Unknown error';
      })
      // Initialize PTZ
      .addCase(initializePTZ.pending, (state) => {
        state.error = null;
      })
      .addCase(initializePTZ.fulfilled, (state) => {
        state.isInitialized = true;
        state.error = null;
      })
      .addCase(initializePTZ.rejected, (state, action) => {
        state.error = action.payload as string;
      })
      // Stop PTZ
      .addCase(stopPTZ.pending, (state) => {
        state.error = null;
      })
      .addCase(stopPTZ.fulfilled, (state) => {
        state.error = null;
      })
      .addCase(stopPTZ.rejected, (state, action) => {
        state.error = action.payload as string;
      })
      // Move PTZ
      .addCase(movePTZ.pending, (state) => {
        state.error = null;
      })
      .addCase(movePTZ.fulfilled, (state) => {
        state.error = null;
      })
      .addCase(movePTZ.rejected, (state, action) => {
        state.error = action.payload as string;
      });
  },
});

export const {
  setStatus,
  setError,
  clearError,
  setInitialized,
} = onvifSlice.actions;

export default onvifSlice.reducer;
