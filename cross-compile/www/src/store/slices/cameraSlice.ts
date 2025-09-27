import { createSlice, PayloadAction } from '@reduxjs/toolkit';
import type { CameraState } from '../../types';

const initialState: CameraState = {
  ip: '192.168.1.100',
  name: 'Keller Kamera 1',
  type: 'Standard',
  isConnected: false,
  snapshotUrl: '',
  isPlaying: true,
  lastSnapshotTime: null,
};

const cameraSlice = createSlice({
  name: 'camera',
  initialState,
  reducers: {
    setCameraIP: (state, action: PayloadAction<string>) => {
      state.ip = action.payload;
    },
    setCameraInfo: (state, action: PayloadAction<{ name: string; type: string }>) => {
      state.name = action.payload.name;
      state.type = action.payload.type;
    },
    setConnectionStatus: (state, action: PayloadAction<boolean>) => {
      state.isConnected = action.payload;
    },
    setSnapshotUrl: (state, action: PayloadAction<string>) => {
      state.snapshotUrl = action.payload;
      state.lastSnapshotTime = new Date();
    },
    togglePlayPause: (state) => {
      state.isPlaying = !state.isPlaying;
    },
    setPlaying: (state, action: PayloadAction<boolean>) => {
      state.isPlaying = action.payload;
    },
    updateSnapshotTime: (state) => {
      state.lastSnapshotTime = new Date();
    },
  },
});

export const {
  setCameraIP,
  setCameraInfo,
  setConnectionStatus,
  setSnapshotUrl,
  togglePlayPause,
  setPlaying,
  updateSnapshotTime,
} = cameraSlice.actions;

export default cameraSlice.reducer;
