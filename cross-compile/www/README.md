# Anyka Camera WebUI

React-based web interface for the Anyka AK3918 camera, communicating securely via ONVIF and custom HTTP endpoints.

## 🚀 Quick Start

### Prerequisites

- Node.js 18+
- npm 9+

### Installation

```bash
cd cross-compile/www
npm install
```

### Development Server

Run the development server with hot reload:

```bash
npm run dev
```

The UI will be available at `http://localhost:3000`.

### Connecting to a Real Device

By default, the dev server proxies requests to `http://192.168.1.100:8080`. To connect to your specific camera IP:

```bash
# Replace with your camera's IP
VITE_API_TARGET=http://192.168.1.50:8080 npm run dev
```

### Building for Production

Build the artifacts for the SD card:

```bash
npm run build
```

The output will be generated in `../../SD_card_contents/anyka_hack/onvif/www`.

## 🛠️ Tech Stack

- **Framework**: React 18 + Vite -> React 19 + Vite 7
- **Styling**: Tailwind CSS v4
- **State Management**: React Query (TanStack Query)
- **Routing**: React Router v7
- **Icons**: Lucide React
- **Protocol**: ONVIF (SOAP over HTTP)

## 🧪 Testing

```bash
# Run unit tests
npm run test

# Run linting
npm run lint
```
