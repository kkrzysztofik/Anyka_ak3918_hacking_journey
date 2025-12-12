#!/usr/bin/env python3
"""
Script to export screenshots from Figma design.
Requires FIGMA_ACCESS_TOKEN environment variable to be set.

Usage:
    export FIGMA_ACCESS_TOKEN=your_token_here
    python3 export_figma_screenshots.py
"""

import os
import requests
import sys
from pathlib import Path

# Figma file configuration
FILE_KEY = 'T9PbSYBLNVSnr6pUXWoVNq'

# Mapping of output filenames to Figma node IDs
SCREENS = {
    'login_figma.png': '24:5136',
    'dashboard_figma.png': '24:1294',
    'settings_identification_figma.png': '24:6206',
    'about_modal_figma.png': '24:6431',
    'diagnostics_figma.png': '21:648',
    'settings_network_figma.png': '24:2006',
    'settings_time_figma.png': '24:2546',
    'settings_maintenance_figma.png': '24:3168',
    'settings_imaging_figma.png': '24:3394',
    'settings_user_management_figma.png': '24:3732',
    'settings_profiles_figma.png': '24:3963',
    'profiles_modal_figma.png': '24:4505',
    'user_modal_figma.png': '24:5171',
    'current_user_popup_figma.png': '24:5721',
    'change_password_modal_figma.png': '24:5959',
}

def export_images():
    """Export images from Figma using the API."""
    token = os.environ.get('FIGMA_ACCESS_TOKEN')
    if not token:
        print("ERROR: FIGMA_ACCESS_TOKEN environment variable not set.")
        print("\nTo get your token:")
        print("1. Go to https://www.figma.com/developers/api#access-tokens")
        print("2. Generate a personal access token")
        print("3. Set it: export FIGMA_ACCESS_TOKEN=your_token_here")
        return False

    # Ensure .ai/img directory exists
    output_dir = Path('.ai/img')
    output_dir.mkdir(parents=True, exist_ok=True)

    # Step 1: Get export URLs from Figma API
    # Note: API accepts node IDs with colons
    node_ids = list(SCREENS.values())
    export_url = f"https://api.figma.com/v1/images/{FILE_KEY}"
    headers = {'X-Figma-Token': token}
    params = {
        'ids': ','.join(node_ids),
        'format': 'png',
        'scale': '2',  # 2x resolution
        'use_absolute_bounds': 'true',  # Export only the frame bounds, not the entire canvas
    }

    print(f"Requesting export URLs from Figma API...")
    response = requests.get(export_url, headers=headers, params=params)

    if response.status_code != 200:
        print(f"ERROR: API request failed with status {response.status_code}")
        print(f"Response: {response.text}")
        return False

    data = response.json()
    if 'error' in data:
        print(f"ERROR: {data['error']}")
        return False

    # Step 2: Download each image
    images = data.get('images', {})
    success_count = 0

    for filename, node_id in SCREENS.items():
        # API returns node IDs with colons, not dashes
        image_url = images.get(node_id)

        if not image_url:
            print(f"WARNING: No export URL for {filename} (node {node_id})")
            continue

        print(f"Downloading {filename}...")
        img_response = requests.get(image_url)

        if img_response.status_code == 200:
            output_path = output_dir / filename
            output_path.write_bytes(img_response.content)
            print(f"  ✓ Saved to {output_path}")
            success_count += 1
        else:
            print(f"  ✗ Failed to download (status {img_response.status_code})")

    print(f"\nExported {success_count}/{len(SCREENS)} images successfully.")
    return success_count == len(SCREENS)

if __name__ == '__main__':
    success = export_images()
    sys.exit(0 if success else 1)
