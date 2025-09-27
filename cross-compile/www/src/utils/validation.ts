/**
 * @file validation.ts
 * @brief Comprehensive input validation utilities
 * @author ONVIF Camera Web Interface
 * @date 2024
 */

/**
 * @brief Validation result interface
 */
export interface ValidationResult {
  isValid: boolean;
  error?: string;
}

/**
 * @brief Validation utilities for common input types
 */
export class ValidationUtils {
  /**
   * @brief Validate IP address format
   * @param ip IP address string to validate
   * @returns ValidationResult with validation status
   */
  static validateIPAddress(ip: string): ValidationResult {
    if (!ip || typeof ip !== 'string') {
      return {
        isValid: false,
        error: 'IP address is required and must be a string'
      };
    }

    const ipRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
    if (!ipRegex.test(ip)) {
      return {
        isValid: false,
        error: 'Invalid IP address format'
      };
    }

    return { isValid: true };
  }

  /**
   * @brief Validate port number
   * @param port Port number to validate
   * @returns ValidationResult with validation status
   */
  static validatePort(port: number): ValidationResult {
    if (typeof port !== 'number' || isNaN(port)) {
      return {
        isValid: false,
        error: 'Port must be a valid number'
      };
    }

    if (port < 1 || port > 65535) {
      return {
        isValid: false,
        error: 'Port must be between 1 and 65535'
      };
    }

    return { isValid: true };
  }

  /**
   * @brief Validate PTZ direction
   * @param direction Direction string to validate
   * @returns ValidationResult with validation status
   */
  static validatePTZDirection(direction: string): ValidationResult {
    if (!direction || typeof direction !== 'string') {
      return {
        isValid: false,
        error: 'Direction is required and must be a string'
      };
    }

    const validDirections = [
      'up', 'down', 'left', 'right',
      'left_up', 'right_up', 'left_down', 'right_down'
    ];

    if (!validDirections.includes(direction)) {
      return {
        isValid: false,
        error: `Invalid direction: ${direction}. Valid directions: ${validDirections.join(', ')}`
      };
    }

    return { isValid: true };
  }

  /**
   * @brief Validate profile token
   * @param token Profile token to validate
   * @returns ValidationResult with validation status
   */
  static validateProfileToken(token: string): ValidationResult {
    if (!token || typeof token !== 'string') {
      return {
        isValid: false,
        error: 'Profile token is required and must be a string'
      };
    }

    if (token.length === 0 || token.length > 64) {
      return {
        isValid: false,
        error: 'Profile token must be between 1 and 64 characters'
      };
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    const tokenRegex = /^[a-zA-Z0-9_-]+$/;
    if (!tokenRegex.test(token)) {
      return {
        isValid: false,
        error: 'Profile token can only contain alphanumeric characters, underscores, and hyphens'
      };
    }

    return { isValid: true };
  }

  /**
   * @brief Validate preset name
   * @param name Preset name to validate
   * @returns ValidationResult with validation status
   */
  static validatePresetName(name: string): ValidationResult {
    if (!name || typeof name !== 'string') {
      return {
        isValid: false,
        error: 'Preset name is required and must be a string'
      };
    }

    if (name.length === 0 || name.length > 32) {
      return {
        isValid: false,
        error: 'Preset name must be between 1 and 32 characters'
      };
    }

    // Check for valid characters (alphanumeric, space, underscore, hyphen)
    const nameRegex = /^[a-zA-Z0-9 _-]+$/;
    if (!nameRegex.test(name)) {
      return {
        isValid: false,
        error: 'Preset name can only contain alphanumeric characters, spaces, underscores, and hyphens'
      };
    }

    return { isValid: true };
  }

  /**
   * @brief Validate imaging settings
   * @param settings Imaging settings object to validate
   * @returns ValidationResult with validation status
   */
  static validateImagingSettings(settings: any): ValidationResult {
    if (!settings || typeof settings !== 'object') {
      return {
        isValid: false,
        error: 'Imaging settings must be an object'
      };
    }

    const { brightness, contrast, saturation } = settings;

    // Validate brightness
    if (brightness !== undefined) {
      if (typeof brightness !== 'number' || isNaN(brightness)) {
        return {
          isValid: false,
          error: 'Brightness must be a valid number'
        };
      }
      if (brightness < 0 || brightness > 100) {
        return {
          isValid: false,
          error: 'Brightness must be between 0 and 100'
        };
      }
    }

    // Validate contrast
    if (contrast !== undefined) {
      if (typeof contrast !== 'number' || isNaN(contrast)) {
        return {
          isValid: false,
          error: 'Contrast must be a valid number'
        };
      }
      if (contrast < 0 || contrast > 100) {
        return {
          isValid: false,
          error: 'Contrast must be between 0 and 100'
        };
      }
    }

    // Validate saturation
    if (saturation !== undefined) {
      if (typeof saturation !== 'number' || isNaN(saturation)) {
        return {
          isValid: false,
          error: 'Saturation must be a valid number'
        };
      }
      if (saturation < 0 || saturation > 100) {
        return {
          isValid: false,
          error: 'Saturation must be between 0 and 100'
        };
      }
    }

    return { isValid: true };
  }

  /**
   * @brief Validate camera configuration
   * @param config Camera configuration object to validate
   * @returns ValidationResult with validation status
   */
  static validateCameraConfig(config: any): ValidationResult {
    if (!config || typeof config !== 'object') {
      return {
        isValid: false,
        error: 'Camera configuration must be an object'
      };
    }

    // Validate IP address
    const ipValidation = this.validateIPAddress(config.ip);
    if (!ipValidation.isValid) {
      return ipValidation;
    }

    // Validate ports
    if (config.onvifPort !== undefined) {
      const onvifPortValidation = this.validatePort(config.onvifPort);
      if (!onvifPortValidation.isValid) {
        return {
          isValid: false,
          error: `ONVIF port validation failed: ${onvifPortValidation.error}`
        };
      }
    }

    if (config.snapshotPort !== undefined) {
      const snapshotPortValidation = this.validatePort(config.snapshotPort);
      if (!snapshotPortValidation.isValid) {
        return {
          isValid: false,
          error: `Snapshot port validation failed: ${snapshotPortValidation.error}`
        };
      }
    }

    if (config.rtspPort !== undefined) {
      const rtspPortValidation = this.validatePort(config.rtspPort);
      if (!rtspPortValidation.isValid) {
        return {
          isValid: false,
          error: `RTSP port validation failed: ${rtspPortValidation.error}`
        };
      }
    }

    return { isValid: true };
  }

  /**
   * @brief Validate string input for XSS prevention
   * @param input String input to validate
   * @param maxLength Maximum allowed length
   * @returns ValidationResult with validation status
   */
  static validateStringInput(input: string, maxLength: number = 1000): ValidationResult {
    if (!input || typeof input !== 'string') {
      return {
        isValid: false,
        error: 'Input is required and must be a string'
      };
    }

    if (input.length > maxLength) {
      return {
        isValid: false,
        error: `Input must be no longer than ${maxLength} characters`
      };
    }

    // Check for potentially dangerous content
    const dangerousPatterns = [
      /<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi,
      /javascript:/gi,
      /on\w+\s*=/gi,
      /<iframe\b[^<]*(?:(?!<\/iframe>)<[^<]*)*<\/iframe>/gi,
      /<object\b[^<]*(?:(?!<\/object>)<[^<]*)*<\/object>/gi,
      /<embed\b[^<]*(?:(?!<\/embed>)<[^<]*)*<\/embed>/gi
    ];

    for (const pattern of dangerousPatterns) {
      if (pattern.test(input)) {
        return {
          isValid: false,
          error: 'Input contains potentially dangerous content'
        };
      }
    }

    return { isValid: true };
  }

  /**
   * @brief Validate URL format
   * @param url URL string to validate
   * @returns ValidationResult with validation status
   */
  static validateURL(url: string): ValidationResult {
    if (!url || typeof url !== 'string') {
      return {
        isValid: false,
        error: 'URL is required and must be a string'
      };
    }

    try {
      new URL(url);
      return { isValid: true };
    } catch {
      return {
        isValid: false,
        error: 'Invalid URL format'
      };
    }
  }

  /**
   * @brief Validate numeric range
   * @param value Numeric value to validate
   * @param min Minimum allowed value
   * @param max Maximum allowed value
   * @returns ValidationResult with validation status
   */
  static validateNumericRange(value: number, min: number, max: number): ValidationResult {
    if (typeof value !== 'number' || isNaN(value)) {
      return {
        isValid: false,
        error: 'Value must be a valid number'
      };
    }

    if (value < min || value > max) {
      return {
        isValid: false,
        error: `Value must be between ${min} and ${max}`
      };
    }

    return { isValid: true };
  }
}

/**
 * @brief Factory function to create validation utilities
 * @returns ValidationUtils instance
 */
export const createValidationUtils = (): typeof ValidationUtils => {
  return ValidationUtils;
};
