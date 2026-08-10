/**
 * PTZ Service Tests
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { apiClient } from '@/services/api';
import type { PTZDirection } from '@/services/ptzService';
import {
  continuousMove,
  getPresets,
  gotoHome,
  gotoPreset,
  removePreset,
  sendAuxiliaryCommand,
  setPreset,
  stopMove,
} from '@/services/ptzService';
import { createMockSOAPFaultResponse, createMockSOAPResponse } from '@/test/utils';

// Mock the api module
vi.mock('@/services/api', () => ({
  apiClient: {
    post: vi.fn(),
  },
  ENDPOINTS: {
    ptz: '/onvif/ptz_service',
  },
}));

describe('ptzService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('continuousMove', () => {
    it('should send ContinuousMove with correct velocity for up direction', async () => {
      const mockResponse = createMockSOAPResponse('<ContinuousMoveResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await continuousMove('ProfileToken1', 'up', 100);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('tptz:ContinuousMove');
      expect(callArgs[1]).toContain('ProfileToken1');
      expect(callArgs[1]).toContain('x="0"');
      expect(callArgs[1]).toContain('y="1"');
    });

    it('should send ContinuousMove with correct velocity for left direction', async () => {
      const mockResponse = createMockSOAPResponse('<ContinuousMoveResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await continuousMove('ProfileToken1', 'left', 100);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('x="-1"');
      expect(callArgs[1]).toContain('y="0"');
    });

    it('should send ContinuousMove with correct velocity for down-right diagonal', async () => {
      const mockResponse = createMockSOAPResponse('<ContinuousMoveResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await continuousMove('ProfileToken1', 'down-right', 100);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('x="0.7"');
      expect(callArgs[1]).toContain('y="-0.7"');
    });

    it('should scale velocity by speed percentage', async () => {
      const mockResponse = createMockSOAPResponse('<ContinuousMoveResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await continuousMove('ProfileToken1', 'right', 50);

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('x="0.5"');
      expect(callArgs[1]).toContain('y="0"');
    });

    it('should throw on unknown direction', async () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await expect(continuousMove('ProfileToken1', 'invalid' as any, 50)).rejects.toThrow(
        'Unknown PTZ direction: invalid',
      );
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Move failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(continuousMove('ProfileToken1', 'up', 50)).rejects.toThrow();
    });
  });

  describe('stopMove', () => {
    it('should send Stop command', async () => {
      const mockResponse = createMockSOAPResponse('<StopResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await stopMove('ProfileToken1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('tptz:Stop');
      expect(callArgs[1]).toContain('ProfileToken1');
      expect(callArgs[1]).toContain('<tptz:PanTilt>true</tptz:PanTilt>');
      expect(callArgs[1]).toContain('<tptz:Zoom>true</tptz:Zoom>');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Stop failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(stopMove('ProfileToken1')).rejects.toThrow();
    });
  });

  describe('gotoHome', () => {
    it('should send GotoHomePosition command', async () => {
      const mockResponse = createMockSOAPResponse('<GotoHomePositionResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await gotoHome('ProfileToken1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('tptz:GotoHomePosition');
      expect(callArgs[1]).toContain('ProfileToken1');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Home failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(gotoHome('ProfileToken1')).rejects.toThrow();
    });
  });

  describe('getPresets', () => {
    it('should parse presets list', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetPresetsResponse>
          <Preset token="preset1">
            <Name>Front Door</Name>
          </Preset>
          <Preset token="preset2">
            <Name>Back Yard</Name>
          </Preset>
        </GetPresetsResponse>
      `);
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const presets = await getPresets('ProfileToken1');

      expect(presets).toHaveLength(2);
      expect(presets[0]).toEqual({ token: 'preset1', name: 'Front Door' });
      expect(presets[1]).toEqual({ token: 'preset2', name: 'Back Yard' });
    });

    it('should handle single preset (not array)', async () => {
      const mockResponse = createMockSOAPResponse(`
        <GetPresetsResponse>
          <Preset token="preset1">
            <Name>Front Door</Name>
          </Preset>
        </GetPresetsResponse>
      `);
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const presets = await getPresets('ProfileToken1');

      expect(presets).toHaveLength(1);
      expect(presets[0]).toEqual({ token: 'preset1', name: 'Front Door' });
    });

    it('should return empty array when no presets', async () => {
      const mockResponse = createMockSOAPResponse('<GetPresetsResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const presets = await getPresets('ProfileToken1');

      expect(presets).toEqual([]);
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Get presets failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(getPresets('ProfileToken1')).rejects.toThrow();
    });
  });

  describe('gotoPreset', () => {
    it('should send GotoPreset command', async () => {
      const mockResponse = createMockSOAPResponse('<GotoPresetResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await gotoPreset('ProfileToken1', 'preset1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('tptz:GotoPreset');
      expect(callArgs[1]).toContain('ProfileToken1');
      expect(callArgs[1]).toContain('preset1');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Goto preset failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(gotoPreset('ProfileToken1', 'preset1')).rejects.toThrow();
    });
  });

  describe('setPreset', () => {
    it('should send SetPreset and return token', async () => {
      const mockResponse = createMockSOAPResponse(`
        <SetPresetResponse>
          <PresetToken>preset_new</PresetToken>
        </SetPresetResponse>
      `);
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      const token = await setPreset('ProfileToken1', 'My Preset');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('tptz:SetPreset');
      expect(callArgs[1]).toContain('My Preset');
      expect(token).toBe('preset_new');
    });

    it('should include preset token when updating existing preset', async () => {
      const mockResponse = createMockSOAPResponse(`
        <SetPresetResponse>
          <PresetToken>preset1</PresetToken>
        </SetPresetResponse>
      `);
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await setPreset('ProfileToken1', 'Updated Name', 'preset1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('<tptz:PresetToken>preset1</tptz:PresetToken>');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Set preset failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(setPreset('ProfileToken1', 'My Preset')).rejects.toThrow();
    });
  });

  describe('removePreset', () => {
    it('should send RemovePreset command', async () => {
      const mockResponse = createMockSOAPResponse('<RemovePresetResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await removePreset('ProfileToken1', 'preset1');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('tptz:RemovePreset');
      expect(callArgs[1]).toContain('ProfileToken1');
      expect(callArgs[1]).toContain('preset1');
    });

    it('should throw on SOAP fault', async () => {
      const mockResponse = createMockSOAPFaultResponse('soap:Sender', 'Remove preset failed');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await expect(removePreset('ProfileToken1', 'preset1')).rejects.toThrow();
    });
  });

  describe('sendAuxiliaryCommand', () => {
    it('sends an auxiliary command with the given data', async () => {
      const mockResponse = createMockSOAPResponse('<SendAuxiliaryCommandResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await sendAuxiliaryCommand('Profile1', 'tt:IRLamp|On');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[0]).toBe('/onvif/ptz_service');
      expect(callArgs[1]).toContain('<tptz:AuxiliaryData>tt:IRLamp|On</tptz:AuxiliaryData>');
    });

    it('escapes XML metacharacters in the profile token', async () => {
      const mockResponse = createMockSOAPResponse('<SendAuxiliaryCommandResponse />');
      vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

      await sendAuxiliaryCommand('a&b', 'tt:IRLamp|On');

      const callArgs = vi.mocked(apiClient.post).mock.calls[0];
      expect(callArgs[1]).toContain('a&amp;b');
    });
  });

  describe('direction velocity mapping', () => {
    it.each([
      ['up', 0, 1],
      ['down', 0, -1],
      ['left', -1, 0],
      ['right', 1, 0],
      ['up-left', -0.7, 0.7],
      ['up-right', 0.7, 0.7],
      ['down-left', -0.7, -0.7],
      ['down-right', 0.7, -0.7],
    ])(
      'should map %s direction to pan=%s tilt=%s at 100%% speed',
      async (direction: string, expectedPan, expectedTilt) => {
        const mockResponse = createMockSOAPResponse('<ContinuousMoveResponse />');
        vi.mocked(apiClient.post).mockResolvedValueOnce(mockResponse);

        await continuousMove('ProfileToken1', direction as PTZDirection, 100);

        const callArgs = vi.mocked(apiClient.post).mock.calls[0];
        expect(callArgs[1]).toContain(`x="${expectedPan}"`);
        expect(callArgs[1]).toContain(`y="${expectedTilt}"`);
      },
    );
  });
});
