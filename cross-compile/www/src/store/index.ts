import { configureStore } from '@reduxjs/toolkit';
import cameraReducer from './slices/cameraSlice';
import onvifReducer from './slices/onvifSlice';
import uiReducer from './slices/uiSlice';
import deviceReducer from './slices/deviceSlice';
import systemUtilizationReducer from './slices/systemUtilizationSlice';

export const store = configureStore({
  reducer: {
    camera: cameraReducer,
    onvif: onvifReducer,
    ui: uiReducer,
    device: deviceReducer,
    systemUtilization: systemUtilizationReducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: ['ui/updateTime', 'camera/updateSnapshotTime'],
        ignoredPaths: ['ui.currentTime', 'camera.lastSnapshotTime'],
      },
    }),
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
