import React, { useEffect } from 'react';
import { Provider } from 'react-redux';
import { store } from './store';
import AppRouter from './router';
import { useAppDispatch, useAppSelector } from './hooks/redux';
import { setCameraIP } from './store/slices/cameraSlice';
import { checkONVIFStatus } from './store/slices/onvifSlice';
import { updateTime } from './store/slices/uiSlice';
import { createCameraConfig } from './config/cameraConfig';
import type { CameraConfig } from './types';

const AppContent: React.FC = () => {
  const dispatch = useAppDispatch();
  const camera = useAppSelector((state) => state.camera);

  useEffect(() => {
    // Detect camera IP from current location
    const hostname = window.location.hostname;
    if (hostname && hostname !== 'localhost') {
      dispatch(setCameraIP(hostname));
    }

    // Update time every second
    const timeInterval = setInterval(() => {
      dispatch(updateTime());
    }, 1000);

    // Check ONVIF status
    const cameraConfig = createCameraConfig(camera.ip);
    dispatch(checkONVIFStatus(cameraConfig));

    return () => {
      clearInterval(timeInterval);
    };
  }, [dispatch, camera.ip]);

  return <AppRouter />;
};

const App: React.FC = () => {
  return (
    <Provider store={store}>
      <AppContent />
    </Provider>
  );
};

export default App;
