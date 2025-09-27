import { createSlice, PayloadAction } from '@reduxjs/toolkit';
import type { UIState } from '../../types';

const initialState: UIState = {
  currentTime: new Date(),
  sidebarCollapsed: false,
  fullscreenMode: false,
};

const uiSlice = createSlice({
  name: 'ui',
  reducers: {
    updateTime: (state) => {
      state.currentTime = new Date();
    },
    toggleSidebar: (state) => {
      state.sidebarCollapsed = !state.sidebarCollapsed;
    },
    setSidebarCollapsed: (state, action: PayloadAction<boolean>) => {
      state.sidebarCollapsed = action.payload;
    },
    toggleFullscreen: (state) => {
      state.fullscreenMode = !state.fullscreenMode;
    },
    setFullscreenMode: (state, action: PayloadAction<boolean>) => {
      state.fullscreenMode = action.payload;
    },
  },
  initialState,
});

export const {
  updateTime,
  toggleSidebar,
  setSidebarCollapsed,
  toggleFullscreen,
  setFullscreenMode,
} = uiSlice.actions;

export default uiSlice.reducer;
