import { createSlice, createAsyncThunk, PayloadAction } from '@reduxjs/toolkit';
import { createSystemUtilizationService } from '../../services/systemUtilizationService';
import type { SystemInfo, DataPoint, SystemUtilizationResponse } from '../../types';

// System utilization state interface
export interface SystemUtilizationState {
  systemInfo: SystemInfo | null;
  cpuHistory: DataPoint[];
  tempHistory: DataPoint[];
  isLoading: boolean;
  error: string | null;
  isAutoRefresh: boolean;
  lastUpdated: Date | null;
  connectionStatus: 'connected' | 'disconnected' | 'checking';
}

const initialState: SystemUtilizationState = {
  systemInfo: null,
  cpuHistory: [],
  tempHistory: [],
  isLoading: false,
  error: null,
  isAutoRefresh: true,
  lastUpdated: null,
  connectionStatus: 'disconnected',
};

// Async thunks for system utilization operations
export const fetchSystemInfo = createAsyncThunk(
  'systemUtilization/fetchSystemInfo',
  async (cameraIP: string, { rejectWithValue }) => {
    try {
      const service = createSystemUtilizationService(cameraIP, 8081, 10000);
      const result = await service.getSystemInfo();
      
      if (result.success && result.data) {
        return {
          systemInfo: result.data,
          timestamp: Date.now(),
        };
      } else {
        return rejectWithValue(result.error || 'Failed to fetch system information');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Unknown error');
    }
  }
);

export const testSystemConnection = createAsyncThunk(
  'systemUtilization/testConnection',
  async (cameraIP: string, { rejectWithValue }) => {
    try {
      const service = createSystemUtilizationService(cameraIP, 8081, 10000);
      const result = await service.testConnection();
      
      if (result.success) {
        return 'connected';
      } else {
        return rejectWithValue(result.error || 'Connection failed');
      }
    } catch (error) {
      return rejectWithValue(error instanceof Error ? error.message : 'Connection test failed');
    }
  }
);

const systemUtilizationSlice = createSlice({
  name: 'systemUtilization',
  initialState,
  reducers: {
    // Toggle auto refresh
    toggleAutoRefresh: (state) => {
      state.isAutoRefresh = !state.isAutoRefresh;
    },
    
    // Set auto refresh state
    setAutoRefresh: (state, action: PayloadAction<boolean>) => {
      state.isAutoRefresh = action.payload;
    },
    
    // Clear error
    clearError: (state) => {
      state.error = null;
    },
    
    // Clear history data
    clearHistory: (state) => {
      state.cpuHistory = [];
      state.tempHistory = [];
    },
    
    // Set connection status
    setConnectionStatus: (state, action: PayloadAction<'connected' | 'disconnected' | 'checking'>) => {
      state.connectionStatus = action.payload;
    },
    
    // Add data point to history
    addDataPoint: (state, action: PayloadAction<{ type: 'cpu' | 'temp'; value: number; timestamp: number }>) => {
      const { type, value, timestamp } = action.payload;
      const dataPoint = { timestamp, value };
      
      if (type === 'cpu') {
        state.cpuHistory = [...state.cpuHistory.slice(-19), dataPoint];
      } else {
        state.tempHistory = [...state.tempHistory.slice(-19), dataPoint];
      }
    },
    
    // Reset state
    reset: (state) => {
      return { ...initialState };
    },
  },
  extraReducers: (builder) => {
    builder
      // Fetch System Info
      .addCase(fetchSystemInfo.pending, (state) => {
        state.isLoading = true;
        state.error = null;
        state.connectionStatus = 'checking';
      })
      .addCase(fetchSystemInfo.fulfilled, (state, action) => {
        state.isLoading = false;
        state.systemInfo = action.payload.systemInfo;
        state.lastUpdated = new Date();
        state.connectionStatus = 'connected';
        state.error = null;
        
        // Update history data
        const now = action.payload.timestamp;
        const cpuValue = action.payload.systemInfo.cpu_usage;
        const tempValue = action.payload.systemInfo.cpu_temperature;
        
        state.cpuHistory = [...state.cpuHistory.slice(-19), { timestamp: now, value: cpuValue }];
        state.tempHistory = [...state.tempHistory.slice(-19), { timestamp: now, value: tempValue }];
      })
      .addCase(fetchSystemInfo.rejected, (state, action) => {
        state.isLoading = false;
        state.error = action.payload as string;
        state.connectionStatus = 'disconnected';
      })
      
      // Test Connection
      .addCase(testSystemConnection.pending, (state) => {
        state.connectionStatus = 'checking';
      })
      .addCase(testSystemConnection.fulfilled, (state) => {
        state.connectionStatus = 'connected';
        state.error = null;
      })
      .addCase(testSystemConnection.rejected, (state, action) => {
        state.connectionStatus = 'disconnected';
        state.error = action.payload as string;
      });
  },
});

export const {
  toggleAutoRefresh,
  setAutoRefresh,
  clearError,
  clearHistory,
  setConnectionStatus,
  addDataPoint,
  reset,
} = systemUtilizationSlice.actions;

export default systemUtilizationSlice.reducer;
